use std::collections::HashMap;
use std::path::{Path, PathBuf}; 
use crate::actors::{Actor, ActorHandle}; 
use crate::VoidRes;

struct ExecWorker {
    exe: PathBuf,
    env: HashMap<String, String>,
    
}


impl<Mes: Send> Actor<Mes> for ExecWorker {
    async fn start(&mut self) -> VoidRes {
        todo!()
    }

    async fn stop(&mut self) -> VoidRes {
        todo!()
    }

    async fn process(&mut self, message: Mes) -> VoidRes {
        todo!()
    }
}
