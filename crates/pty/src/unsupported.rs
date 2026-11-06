use crate::{Exit, Failure, Interest, Launch, Ready, Size, Status, Transfer};
use std::time::Duration;

pub enum Process {}

impl Process {
    pub fn spawn(_: Launch) -> Result<Self, Failure> {
        Err(Failure::Unsupported)
    }
    pub fn id(&self) -> u32 {
        match *self {}
    }
    pub fn status(&self) -> Status {
        match *self {}
    }
    pub fn observe(&mut self) -> Result<Option<Exit>, Failure> {
        match *self {}
    }
    pub fn size(&self) -> Result<Size, Failure> {
        match *self {}
    }
    pub fn resize(&self, _: Size) -> Result<(), Failure> {
        match *self {}
    }
    pub fn read(&mut self, _: &mut [u8]) -> Result<Transfer, Failure> {
        match *self {}
    }
    pub fn write(&mut self, _: &[u8]) -> Result<Transfer, Failure> {
        match *self {}
    }
    pub fn poll(&self, _: Interest, _: Duration) -> Result<Ready, Failure> {
        match *self {}
    }
    pub fn retire(&mut self) -> Result<(), Failure> {
        match *self {}
    }
    pub fn force(&mut self) -> Result<(), Failure> {
        match *self {}
    }
    pub fn close(&mut self) -> Result<Exit, Failure> {
        match *self {}
    }
}
