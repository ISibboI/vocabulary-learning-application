{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = {self, nixpkgs, flake-utils, rust-overlay, crane}:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          system = "x86_64-linux";
          overlays = [(import rust-overlay)];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
          rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
          src = lib.cleanSourceWith {
            src = ./.; # The original, unfiltered source
            filter = path: type:
              # Allow sql files for migrations
              (lib.hasSuffix "\.sql" path) ||
              # Default filter from crane (allow .rs files)
              (craneLib.filterCargoSources path type)
            ;
        };
          nativeBuildInputs = with pkgs; [rustToolchain pkg-config];
          buildInputs = with pkgs; [rustToolchain openssl];
          developmentTools = with pkgs; [(diesel-cli.override {sqliteSupport = false; mysqlSupport = false;}) postgresql];
          commonArgs = {
            inherit src buildInputs nativeBuildInputs;
          };
          cargoDebugArtifacts = craneLib.buildDepsOnly(commonArgs // {
            cargoBuildCommand = "cargo build --profile debug";
          });
          debugBin = craneLib.buildPackage(commonArgs // {
            inherit cargoDebugArtifacts;
            cargoBuildCommand = "cargo build --profile debug";
          });
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          bin = craneLib.buildPackage(commonArgs // {inherit cargoArtifacts;});
          dockerImage = pkgs.dockerTools.streamLayeredImage {
            name = "rvoc-backend";
            tag = "latest";
            contents = [bin pkgs.cacert];
            config = {
              Cmd = ["${bin}/bin/rvoc-backend"];
            };
          };
          debugDockerImage = pkgs.dockerTools.streamLayeredImage {
            name = "rvoc-backend";
            tag = "latest";
            contents = [debugBin pkgs.cacert];
            config = {
              Cmd = ["${debugBin}/bin/rvoc-backend"];
            };
          };
        in
        with pkgs;
        {
          packages = {
            inherit bin debugBin dockerImage debugDockerImage;
            default = bin;
          };
          devShells.default = mkShell {
            inputsFrom = [bin];
            buildInputs = with pkgs; [dive wget];
            packages = developmentTools;
            shellHook = ''
              export PGDATA=$PWD/backend/data/postgres_dev_data
              export PGHOST=$PWD/backend/data/postgres_dev
              export LOG_PATH=$PWD/backend/data/postgres_dev/LOG
              export PGDATABASE=rvoc_dev
              export POSTGRES_RVOC_URL="postgresql://''${USER}@/''${PGDATABASE}?host=$PGHOST"
              export DATABASE_URL=$POSTGRES_RVOC_URL
              if [ ! -d $PGHOST ]; then
                mkdir -p $PGHOST
              fi
              if [ ! -d $PGDATA ]; then
                echo 'Initializing postgresql database...'
                initdb $PGDATA --auth=trust >/dev/null
              fi
              pg_ctl start -l $LOG_PATH -o "-c listen_addresses= -c unix_socket_directories=$PGHOST"
            '';
          };
        }

      );
}
