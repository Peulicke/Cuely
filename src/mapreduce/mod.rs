// Cuely is an open source web search engine.
// Copyright (C) 2022 Cuely ApS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::net::SocketAddr;

mod manager;
mod worker;

pub use manager::Manager;
use thiserror::Error;
pub use worker::StatelessWorker;
pub use worker::Worker;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error")]
    IO(#[from] std::io::Error),

    #[error("error while serializing/deserializing to/from bytes")]
    Serialization(#[from] bincode::Error),

    #[error("could not get a working worker")]
    NoAvailableWorker,

    #[error("did not get a reponse")]
    NoResponse,
}

pub trait Map<W, T>
where
    Self: Serialize + DeserializeOwned + Send,
    T: Serialize + DeserializeOwned + Send,
    W: Worker,
{
    fn map(self, worker: &W) -> T;
}

pub trait Reduce<T> {
    #[must_use]
    fn reduce(self, element: T) -> Self;
}

#[allow(clippy::trait_duplication_in_bounds)]
pub trait MapReduce<W, I, O1, O2>
where
    Self: Sized + Iterator<Item = I> + Send,
    W: Worker,
    I: Map<W, O1>,
    O1: Serialize + DeserializeOwned + Send,
    O2: From<O1> + Reduce<O1> + Send + Reduce<O2>,
{
    fn map_reduce(self, workers: &[SocketAddr]) -> Option<O2> {
        let manager = Manager::new(workers);
        manager.run(self)
    }
}

#[allow(clippy::trait_duplication_in_bounds)]
impl<W, I, O1, O2, T> MapReduce<W, I, O1, O2> for T
where
    W: Worker,
    T: Iterator<Item = I> + Sized + Send,
    I: Map<W, O1>,
    O1: Serialize + DeserializeOwned + Send,
    O2: From<O1> + Reduce<O1> + Send + Reduce<O2>,
{
}

#[derive(Serialize, Deserialize, Debug)]
enum Task<T> {
    Job(T),
    AllFinished,
}
