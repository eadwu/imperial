{
  description = "`Imperial` flake shim for `native` NixOS compat";

  # Flake for compatibility with non-flake commands
  inputs.flake-compat = { type = "github"; owner = "edolstra"; repo = "flake-compat"; flake = false; };

  # Nixpkgs Channels
  inputs.nixpkgs = { type = "github"; owner = "NixOS"; repo = "nixpkgs"; };

  outputs = { self, nixpkgs, ... }@inputs:
    let
      supportedSystems = [ "x86_64-linux" ];

      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);

      overlays = builtins.attrValues self.overlays;

      nixpkgsFor = forAllSystems (
        system: import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true;
        }
      );
    in
    {
      overlay =
        with nixpkgs.lib;
        foldl'
          (final': prev': composeExtensions final' prev')
          (final: prev: { })
          overlays;

      overlays.default = final: prev:
        with final.pkgs;
        {

          rorbind = rustPlatform.buildRustPackage {
            pname = "rorbind";
            version = builtins.substring 0 8 self.lastModifiedDate;

            src = self;
            cargoLock.lockFile = "${self}/Cargo.lock";

            # Build all components (binary and library)
            cargoBuildFlags = [ "--workspace" ];
          };

        };

      defaultPackage = forAllSystems (system: self.packages.${system}.rorbind);
      packages = forAllSystems (
        system:
        let
          pkgSet = nixpkgsFor.${system};
        in
        {
          inherit (pkgSet)
            rorbind
            ;
        }
      );

      checks = forAllSystems (system: self.packages.${system});
    };
}
