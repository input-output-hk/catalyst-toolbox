{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:kreisys/flake-utils";
  };
  outputs = { self, nixpkgs, utils }:
    utils.lib.simpleFlake rec {
      inherit nixpkgs;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      preOverlays = [ overlay ];
      overlay = final: prev: {
        catalyst-toolbox = prev.rustPlatform.buildRustPackage {
          inherit ((builtins.fromTOML
            (builtins.readFile (./Cargo.toml))).package)
            name version;
          src = ./.;
          cargoSha256 = "sha256-43Ccz4h1XDhcL9rcJlJn0StkHMNqu1jgcM3HGak8sd4=";
          nativeBuildInputs = with final; [ pkg-config protobuf rustfmt ];
          buildInputs = with final; [ openssl ];
          PROTOC = "${final.protobuf}/bin/protoc";
          PROTOC_INCLUDE = "${final.protobuf}/include";
        };
      };
      packages = { catalyst-toolbox }@pkgs: pkgs;
      devShell =
        { mkShell, rustc, cargo, pkg-config, openssl, protobuf, rustfmt }:
        mkShell {
          PROTOC = "${protobuf}/bin/protoc";
          PROTOC_INCLUDE = "${protobuf}/include";
          buildInputs = [ rustc cargo pkg-config openssl protobuf rustfmt ];
        };
    };
}
