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
          cargoSha256 = "sha256-LoO4+xM2alSYh+b3Ov9vNL84vbUDXcGNiA0E5x5CC5c";
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
