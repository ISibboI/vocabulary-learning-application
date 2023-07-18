mod configuration;
mod error;
mod main;

// The actual main function is in the main submodule,
// because otherwise CLion's automatic generation of use-statements sometimes
// imports from the crate root instead of the actual location of a declaration.
// Since we are also building a library as well that has no imports on its root,
// those imports from the crate root then fail to resolve.
fn main() {
    main::main();
}
