mod arch;
mod cli;
mod exec;
mod guest;
mod ident;
mod install;
mod json;
mod mem;
mod meta;
mod path;
mod scratch;
mod sha256;
mod sys;
mod tracer;
pub fn run() {
    cli::main()
}
