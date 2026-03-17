use jackdaw_macros::*;

event_decl!(e1, "timer completed");
event_decl!(e2, "timer's responsibility transferred");

#[raven::spec]
#[raven::pollable(Timer)]
#[raven::complete(e1)]
#[raven::resp_transfer(e2)]
pub fn timer_spec()  {
    ignore_path!("spec")
}
