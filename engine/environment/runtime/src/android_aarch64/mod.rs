mod arch;
#[path = "../cli.rs"]
mod cli;
#[path = "../exec.rs"]
mod exec;
mod guest;
#[path = "../ident.rs"]
mod ident;
mod install;
#[path = "../json.rs"]
mod json;
#[path = "../mem.rs"]
mod mem;
mod meta;
#[path = "../path.rs"]
mod path;
#[path = "../scratch.rs"]
mod scratch;
#[path = "../sha256.rs"]
mod sha256;
mod sys;
mod tracer;
pub fn run() {
    cli::main()
}
