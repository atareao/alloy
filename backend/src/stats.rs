use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::Json,
    routing::{get, post},
    Router,
};
use bollard::{
    container::PruneContainersOptions,
    container::{ListContainersOptions, StatsOptions},
    image::PruneImagesOptions,
    network::{ListNetworksOptions, PruneNetworksOptions},
    volume::{ListVolumesOptions, PruneVolumesOptions},
    Docker,
};
use std::convert::Infallible;
use std::time::Duration;
use std::time::Instant;

use crate::containers::find_container_by_name;
use crate::models::*;
use crate::state::AppState;

pub fn calc_container_stats(name: &str, stats: &bollard::container::Stats) -> ContainerStats {
    let cpu_delta = stats
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(stats.precpu_stats.cpu_usage.total_usage);
    let system_delta = stats
        .cpu_stats
        .system_cpu_usage
        .unwrap_or(0)
        .saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0));
    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
    let cpu_percent = if system_delta > 0 {
        (cpu_delta as f64 / system_delta as f64) * num_cpus * 100.0
    } else {
        0.0
    };
    rest_of_stats(stats, name, cpu_percent)
}

fn calc_container_stats_cached(
    name: &str,
    stats: &bollard::container::Stats,
    cache: &CpuStatsCache,
    interval_ns: u64,
) -> ContainerStats {
    let mut cache = cache.lock().unwrap();
    let prev = cache.entry(name.to_string()).or_default();

    let cpu_delta = stats
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(prev.total_usage);

    let cpu_percent = if interval_ns > 0 && cpu_delta > 0 {
        (cpu_delta as f64 / interval_ns as f64) * 100.0
    } else {
        0.0
    };

    prev.total_usage = stats.cpu_stats.cpu_usage.total_usage;

    rest_of_stats(stats, name, cpu_percent)
}

fn rest_of_stats(
    stats: &bollard::container::Stats,
    name: &str,
    cpu_percent: f64,
) -> ContainerStats {
    let mem_usage = stats.memory_stats.usage.unwrap_or(0) as f64;
    let mem_limit = stats.memory_stats.limit.unwrap_or(1) as f64;
    let memory_usage_mb = mem_usage / 1_048_576.0;
    let memory_limit_mb = mem_limit / 1_048_576.0;
    let (rx, tx) = stats
        .networks
        .as_ref()
        .map(|nets| {
            nets.values().fold((0u64, 0u64), |(rx, tx), net| {
                (rx + net.rx_bytes, tx + net.tx_bytes)
            })
        })
        .unwrap_or((0, 0));
    ContainerStats {
        name: name.to_string(),
        cpu_percent: (cpu_percent * 100.0).round() / 100.0,
        memory_usage_mb: (memory_usage_mb * 100.0).round() / 100.0,
        memory_limit_mb: (memory_limit_mb * 100.0).round() / 100.0,
        network_rx_kb: (rx as f64 / 1024.0 * 100.0).round() / 100.0,
        network_tx_kb: (tx as f64 / 1024.0 * 100.0).round() / 100.0,
    }
}

async fn get_container_stats(
    State(docker): State<Docker>,
    State(cache): State<CpuStatsCache>,
    Path(name): Path<String>,
) -> Result<Json<ContainerStats>, AppError> {
    let container = find_container_by_name(&docker, &name).await?;
    let cid = container
        .id
        .as_deref()
        .ok_or_else(|| AppError::NotFound("no container id".into()))?;
    let mut stats_stream = docker.stats(cid, None::<StatsOptions>);
    let first = futures::StreamExt::next(&mut stats_stream)
        .await
        .ok_or_else(|| AppError::Internal("empty stats stream".into()))?
        .map_err(|e| AppError::Docker(e.to_string()))?;
    {
        let mut c = cache.lock().unwrap();
        c.entry(name.clone()).or_default().total_usage = first.cpu_stats.cpu_usage.total_usage;
    }
    tokio::time::sleep(Duration::from_millis(800)).await;
    let second = futures::StreamExt::next(&mut stats_stream)
        .await
        .ok_or_else(|| {
            let mut c = cache.lock().unwrap();
            c.remove(&name);
            AppError::Internal("second stats sample unavailable".into())
        })?
        .map_err(|e| AppError::Docker(e.to_string()))?;
    Ok(Json(calc_container_stats_cached(
        &name,
        &second,
        &cache,
        800_000_000,
    )))
}

#[allow(clippy::type_complexity)]
async fn sse_stats_events_h(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let docker = state.docker.clone();
    let cache = state.prev_cpu_stats.clone();
    use futures::stream::unfold;
    let stream = unfold(
        (docker, cache, Instant::now()),
        |(docker, cache, last_time)| async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let now = Instant::now();
            let interval_ns = now.duration_since(last_time).as_nanos() as u64;
            let running = docker
                .list_containers(Some(ListContainersOptions::<String> {
                    all: false,
                    ..Default::default()
                }))
                .await
                .unwrap_or_default();
            let mut stats = vec![];
            for c in &running {
                let name = c
                    .names
                    .as_ref()
                    .and_then(|n| n.first())
                    .map(|n| strip_name(n))
                    .unwrap_or_default();
                if let Some(cid) = c.id.as_deref() {
                    let mut stream = docker.stats(cid, None::<StatsOptions>);
                    if let Some(Ok(s)) = futures::StreamExt::next(&mut stream).await {
                        stats.push(calc_container_stats_cached(&name, &s, &cache, interval_ns));
                    }
                }
            }
            let evt = Event::default().event("stats").json_data(stats).unwrap();
            Some((Ok(evt), (docker, cache, now)))
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn prune_all_h(State(docker): State<Docker>) -> Result<Json<PruneResult>, AppError> {
    let mut total_space: u64 = 0;
    let containers_pruned = match docker
        .prune_containers(None::<PruneContainersOptions<String>>)
        .await
    {
        Ok(report) => {
            let count = report
                .containers_deleted
                .as_ref()
                .map(|v| v.len() as u64)
                .unwrap_or(0);
            total_space += report.space_reclaimed.unwrap_or(0) as u64;
            count
        }
        Err(e) => {
            tracing::error!("prune containers: {}", e);
            0
        }
    };
    let images_pruned = match docker
        .prune_images(None::<PruneImagesOptions<String>>)
        .await
    {
        Ok(report) => {
            let count = report
                .images_deleted
                .as_ref()
                .map(|v| v.len() as u64)
                .unwrap_or(0);
            total_space += report.space_reclaimed.unwrap_or(0) as u64;
            count
        }
        Err(e) => {
            tracing::error!("prune images: {}", e);
            0
        }
    };
    let networks_pruned = match docker
        .prune_networks(None::<PruneNetworksOptions<String>>)
        .await
    {
        Ok(report) => report
            .networks_deleted
            .as_ref()
            .map(|v| v.len() as u64)
            .unwrap_or(0),
        Err(e) => {
            tracing::error!("prune networks: {}", e);
            0
        }
    };
    let volumes_pruned = match docker
        .prune_volumes(None::<PruneVolumesOptions<String>>)
        .await
    {
        Ok(report) => {
            let count = report
                .volumes_deleted
                .as_ref()
                .map(|v| v.len() as u64)
                .unwrap_or(0);
            total_space += report.space_reclaimed.unwrap_or(0) as u64;
            count
        }
        Err(e) => {
            tracing::error!("prune volumes: {}", e);
            0
        }
    };
    Ok(Json(PruneResult {
        containers_pruned,
        images_pruned,
        networks_pruned,
        volumes_pruned,
        space_reclaimed_bytes: total_space,
    }))
}

async fn list_volumes_h(State(docker): State<Docker>) -> Result<Json<Vec<VolumeInfo>>, AppError> {
    let resp = docker
        .list_volumes(None::<ListVolumesOptions<String>>)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let volumes = resp
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| VolumeInfo {
            name: v.name,
            driver: v.driver,
            mountpoint: v.mountpoint,
            size: None,
        })
        .collect();
    Ok(Json(volumes))
}

async fn list_networks_h(State(docker): State<Docker>) -> Result<Json<Vec<NetworkInfo>>, AppError> {
    let nets = docker
        .list_networks(None::<ListNetworksOptions<String>>)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result: Vec<NetworkInfo> = nets
        .into_iter()
        .map(|n| {
            let subnet = n.ipam.as_ref().and_then(|ipam| {
                ipam.config
                    .as_ref()
                    .and_then(|cfg| cfg.first())
                    .and_then(|c| c.subnet.clone())
            });
            NetworkInfo {
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
                scope: n.scope.unwrap_or_default(),
                subnet,
            }
        })
        .collect();
    Ok(Json(result))
}

async fn docker_info_h(State(docker): State<Docker>) -> Result<Json<DockerInfoResp>, AppError> {
    let info = docker
        .info()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(DockerInfoResp {
        version: info.server_version.unwrap_or_default(),
        os: info.operating_system.unwrap_or_default(),
        arch: info.architecture.unwrap_or_default(),
        containers_total: info.containers.unwrap_or(0),
        containers_running: info.containers_running.unwrap_or(0),
        containers_paused: info.containers_paused.unwrap_or(0),
        containers_stopped: info.containers_stopped.unwrap_or(0),
        images: info.images.unwrap_or(0),
    }))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/stats/{name}", get(get_container_stats))
        .route("/api/stats-events", get(sse_stats_events_h))
        .route("/api/prune", post(prune_all_h))
        .route("/api/volumes", get(list_volumes_h))
        .route("/api/networks", get(list_networks_h))
        .route("/api/docker-info", get(docker_info_h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(
        cpu_delta: u64,
        sys_delta: u64,
        online_cpus: u64,
        mem_usage: u64,
        mem_limit: u64,
    ) -> bollard::container::Stats {
        serde_json::from_value(serde_json::json!({
            "read": "2024-01-01T00:00:00Z",
            "preread": "2024-01-01T00:00:00Z",
            "num_procs": 0,
            "pids_stats": {},
            "networks": {
                "eth0": { "rx_bytes": 0, "tx_bytes": 0, "rx_packets": 0, "tx_packets": 0, "rx_errors": 0, "tx_errors": 0, "rx_dropped": 0, "tx_dropped": 0 }
            },
            "memory_stats": {
                "stats": null,
                "max_usage": 0,
                "usage": mem_usage,
                "failcnt": 0,
                "limit": mem_limit
            },
            "cpu_stats": {
                "cpu_usage": {
                    "total_usage": cpu_delta,
                    "usage_in_kernelmode": 0,
                    "usage_in_usermode": 0
                },
                "system_cpu_usage": sys_delta,
                "online_cpus": online_cpus,
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "precpu_stats": {
                "cpu_usage": {
                    "total_usage": 0,
                    "usage_in_kernelmode": 0,
                    "usage_in_usermode": 0
                },
                "system_cpu_usage": 0,
                "online_cpus": online_cpus,
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "blkio_stats": {},
            "storage_stats": {}
        }))
        .expect("valid stats json")
    }

    fn make_stats_with_network(
        cpu_delta: u64,
        sys_delta: u64,
        online_cpus: u64,
        mem_usage: u64,
        mem_limit: u64,
        rx: u64,
        tx: u64,
    ) -> bollard::container::Stats {
        serde_json::from_value(serde_json::json!({
            "read": "2024-01-01T00:00:00Z",
            "preread": "2024-01-01T00:00:00Z",
            "num_procs": 0,
            "pids_stats": {},
            "networks": {
                "eth0": { "rx_bytes": rx, "tx_bytes": tx, "rx_packets": 0, "tx_packets": 0, "rx_errors": 0, "tx_errors": 0, "rx_dropped": 0, "tx_dropped": 0 }
            },
            "memory_stats": {
                "stats": null,
                "max_usage": 0,
                "usage": mem_usage,
                "failcnt": 0,
                "limit": mem_limit
            },
            "cpu_stats": {
                "cpu_usage": {
                    "total_usage": cpu_delta,
                    "usage_in_kernelmode": 0,
                    "usage_in_usermode": 0
                },
                "system_cpu_usage": sys_delta,
                "online_cpus": online_cpus,
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "precpu_stats": {
                "cpu_usage": {
                    "total_usage": 0,
                    "usage_in_kernelmode": 0,
                    "usage_in_usermode": 0
                },
                "system_cpu_usage": 0,
                "online_cpus": online_cpus,
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "blkio_stats": {},
            "storage_stats": {}
        }))
        .expect("valid stats json")
    }

    #[test]
    fn test_calc_container_stats_zero_cpu() {
        let s = make_stats(100, 1000, 2, 1_048_576, 2_097_152);
        let result = calc_container_stats("test", &s);
        assert_eq!(result.name, "test");
        assert!((result.cpu_percent - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_container_stats_cpu_50_percent() {
        let s = make_stats(200, 1000, 2, 1_048_576, 2_097_152);
        let result = calc_container_stats("test", &s);
        assert!((result.cpu_percent - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_container_stats_memory() {
        let s = make_stats(100, 1000, 1, 1_048_576, 2_097_152);
        let result = calc_container_stats("test", &s);
        assert!((result.memory_usage_mb - 1.0).abs() < 0.01);
        assert!((result.memory_limit_mb - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_container_stats_network() {
        let s = make_stats_with_network(100, 1000, 1, 1_048_576, 2_097_152, 2048, 4096);
        let result = calc_container_stats("test", &s);
        assert!((result.network_rx_kb - 2.0).abs() < 0.01);
        assert!((result.network_tx_kb - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_calc_container_stats_no_system_cpu() {
        let json = serde_json::json!({
            "read": "2024-01-01T00:00:00Z",
            "preread": "2024-01-01T00:00:00Z",
            "num_procs": 0,
            "pids_stats": {},
            "networks": {},
            "memory_stats": { "stats": null, "max_usage": 0, "usage": 0, "failcnt": 0, "limit": 0 },
            "cpu_stats": {
                "cpu_usage": { "total_usage": 100, "usage_in_kernelmode": 0, "usage_in_usermode": 0 },
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "precpu_stats": {
                "cpu_usage": { "total_usage": 0, "usage_in_kernelmode": 0, "usage_in_usermode": 0 },
                "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "blkio_stats": {},
            "storage_stats": {}
        });
        let s: bollard::container::Stats = serde_json::from_value(json).expect("valid stats");
        let result = calc_container_stats("test", &s);
        assert_eq!(result.cpu_percent, 0.0);
    }

    #[test]
    fn test_calc_container_stats_no_memory_stats() {
        let json = serde_json::json!({
            "read": "2024-01-01T00:00:00Z",
            "preread": "2024-01-01T00:00:00Z",
            "num_procs": 0,
            "pids_stats": {},
            "networks": {},
            "memory_stats": { "stats": null, "max_usage": 0, "usage": null, "failcnt": 0, "limit": null },
            "cpu_stats": {
                "cpu_usage": { "total_usage": 0, "usage_in_kernelmode": 0, "usage_in_usermode": 0 },
                "system_cpu_usage": 0, "online_cpus": 1, "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "precpu_stats": {
                "cpu_usage": { "total_usage": 0, "usage_in_kernelmode": 0, "usage_in_usermode": 0 },
                "system_cpu_usage": 0, "online_cpus": 1, "throttling_data": { "periods": 0, "throttled_periods": 0, "throttled_time": 0 }
            },
            "blkio_stats": {},
            "storage_stats": {}
        });
        let s: bollard::container::Stats = serde_json::from_value(json).expect("valid stats");
        let result = calc_container_stats("test", &s);
        assert_eq!(result.memory_usage_mb, 0.0);
    }
}
