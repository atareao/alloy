import { useEffect, useState, useMemo } from "react";
import { useMediaQuery } from "@mantine/hooks";
import {
  Badge, Button, Container, Group, Loader, Paper, Table, Text,
  Title, Stack, TextInput, ActionIcon, Tooltip, Code, SimpleGrid,
} from "@mantine/core";
import type { ImageInfo } from "../types";
import { apiFetch, truncate } from "../api";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return (bytes / Math.pow(1024, i)).toFixed(2) + " " + units[i];
}

function formatTimestamp(unix: number): string {
  const d = new Date(unix * 1000);
  return d.toLocaleDateString("es-ES", {
    year: "numeric", month: "short", day: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}

export default function ImagesPage() {
  const isMobile = useMediaQuery("(max-width: 768px)");
  const [images, setImages] = useState<ImageInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    apiFetch("/api/images")
      .then((res) => res.json())
      .then((data) => { setImages(data); setLoading(false); })
      .catch(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <Container py="xl">
        <Group justify="center">
          <Loader />
          <Text>Cargando imágenes...</Text>
        </Group>
      </Container>
    );
  }

  const q = searchQuery.toLowerCase().trim();
  const filtered = q
    ? images.filter((img) =>
        img.repo.toLowerCase().includes(q) ||
        img.tag.toLowerCase().includes(q) ||
        img.id.toLowerCase().includes(q) ||
        img.repo_tags.some((t) => t.toLowerCase().includes(q))
      )
    : images;

  const totalSize = useMemo(
    () => images.reduce((acc, img) => acc + img.size_mb, 0),
    [images]
  );

  return (
    <>
      <Paper shadow="sm" p="md" mb="md" withBorder>
        <Stack gap="sm">
          <Group justify="space-between" wrap="wrap">
            <Title order={4}>📦 Imágenes Docker</Title>
            <Group gap="xs">
              <Badge size="lg" variant="light" color="blue">{images.length} imágenes</Badge>
              <Badge size="lg" variant="light" color="grape">{formatBytes(totalSize * 1_048_576)}</Badge>
            </Group>
          </Group>
          <TextInput
            placeholder="Buscar por repositorio, tag o ID..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.currentTarget.value)}
            rightSection={searchQuery ? (
              <ActionIcon variant="subtle" size="sm" onClick={() => setSearchQuery("")}>✕</ActionIcon>
            ) : undefined}
          />
        </Stack>
      </Paper>

      {isMobile ? (
        <Stack gap="sm">
          {filtered.map((img) => (
            <Paper key={img.id + img.repo + img.tag} shadow="sm" p="sm" withBorder>
              <Stack gap="xs">
                <Group justify="space-between" wrap="nowrap">
                  <Text size="sm" fw={500} truncate style={{ flex: 1 }}>{img.repo}:{img.tag}</Text>
                  <Code size="xs">{img.id}</Code>
                </Group>
                <SimpleGrid cols={2} spacing="xs">
                  <Stack gap={0}>
                    <Text size="xs" c="dimmed">Tamaño</Text>
                    <Text size="sm">{formatBytes(img.size_mb * 1_048_576)}</Text>
                  </Stack>
                  <Stack gap={0}>
                    <Text size="xs" c="dimmed">Virtual</Text>
                    <Text size="sm">{formatBytes(img.virtual_size_mb * 1_048_576)}</Text>
                  </Stack>
                  <Stack gap={0}>
                    <Text size="xs" c="dimmed">Creado</Text>
                    <Text size="xs">{formatTimestamp(img.created)}</Text>
                  </Stack>
                  <Stack gap={0}>
                    <Text size="xs" c="dimmed">Contenedores</Text>
                    <Text size="sm">{img.containers}</Text>
                  </Stack>
                </SimpleGrid>
                {img.repo_tags.length > 1 && (
                  <Stack gap={2}>
                    <Text size="xs" c="dimmed">Tags adicionales</Text>
                    {img.repo_tags.slice(1).map((t, i) => (
                      <Text key={i} size="xs"><Code>{t}</Code></Text>
                    ))}
                  </Stack>
                )}
              </Stack>
            </Paper>
          ))}
        </Stack>
      ) : (
        <Paper shadow="sm" withBorder>
          <Table.ScrollContainer minWidth={800}>
            <Table striped highlightOnHover>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>ID</Table.Th>
                  <Table.Th>Repositorio</Table.Th>
                  <Table.Th>Tag</Table.Th>
                  <Table.Th>Tamaño</Table.Th>
                  <Table.Th>Virtual</Table.Th>
                  <Table.Th>Creado</Table.Th>
                  <Table.Th>Cont</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {filtered.map((img) => (
                  <Table.Tr key={img.id + img.repo + img.tag}>
                    <Table.Td><Code size="xs">{img.id}</Code></Table.Td>
                    <Table.Td>
                      <Text size="sm" fw={500}>{img.repo}</Text>
                    </Table.Td>
                    <Table.Td>
                      <Badge size="sm" variant="light" color="gray">{img.tag}</Badge>
                    </Table.Td>
                    <Table.Td><Text size="sm">{formatBytes(img.size_mb * 1_048_576)}</Text></Table.Td>
                    <Table.Td><Text size="sm">{formatBytes(img.virtual_size_mb * 1_048_576)}</Text></Table.Td>
                    <Table.Td><Text size="xs">{formatTimestamp(img.created)}</Text></Table.Td>
                    <Table.Td>
                      <Badge size="sm" variant="light" color={img.containers > 0 ? "green" : "dimmed"}>
                        {img.containers}
                      </Badge>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          </Table.ScrollContainer>
        </Paper>
      )}

      {filtered.length === 0 && (
        <Paper shadow="sm" p="xl" withBorder>
          <Text ta="center" c="dimmed">
            {searchQuery ? "No se encontraron imágenes con ese criterio" : "No hay imágenes disponibles"}
          </Text>
        </Paper>
      )}
    </>
  );
}