user     := "atareao"
name     := "alloy"
version  := `vampus show`


list:
    @just --list

lint:
    cd backend && cargo clippy --all-targets --all-features

fmt:
    cd backend && cargo fmt -- --check

fmt-fix:
    cd backend && cargo fmt

build:
    @podman build \
        --tag={{user}}/{{name}}:{{version}} \
        --tag={{user}}/{{name}}:latest .

push:
    @podman image push {{user}}/{{name}}:{{version}}
    @podman image push {{user}}/{{name}}:latest

upgrade:
    #!/bin/fish
    vampus upgrade --patch
    set VERSION $(vampus show)
    cd backend && cargo update
    git commit -am "Upgrade to version $VERSION"
    git tag -a "$VERSION" -m "Version $VERSION"
    # clean old podman images
    podman image list  | grep {{name}} | sort -r | tail -n +5 | awk '{print $3}' | while read id; echo $id; podman rmi $id; end
    just build push
