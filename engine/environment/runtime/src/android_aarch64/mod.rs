mod arch;
mod cli;
mod exec;
mod guest;
mod ident;
mod install;
#[path = "../json.rs"]
mod json;
#[path = "../mem.rs"]
mod mem;
mod meta;
mod path;
mod scratch;
#[path = "../sha256.rs"]
mod sha256;
mod sys;
mod tracer;
pub fn run() {
    cli::main()
}
