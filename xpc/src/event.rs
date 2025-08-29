use serde::{Deserialize, Serialize};

pub trait XPCEvent<'a>: Serialize + Deserialize<'a> {}

