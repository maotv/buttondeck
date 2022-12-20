
use midir::{MidiInput, MidiOutput};
use thiserror::Error;

#[derive(Error,Debug)]
pub enum DeckError {
    #[error("message `{0}`")]
    Message(String),
    #[error("button ref error")]
    InvalidRef,
    #[error("button key error {0}")]
    InvalidKey(String),
    #[error("no directory")]
    NoDirectory,
    #[error("no device")]
    NoDevice,
    #[error("io error: `{0}`")]
    IOError(#[from] std::io::Error),
    #[error("hid error: `{0}`")]
    HidError(#[from] hidapi::HidError),
    #[error("serde error: `{0}`")]
    SerdeError(#[from] serde_json::Error),
    #[error("streamdeck error: `{0}`")]
    StreamdeckError(#[from] streamdeck::Error),
    #[error("no hid api")]
    NoHidApi,
    #[error("MidiSendError")]
    MidiSendError(#[from] midir::InitError),
    #[error("MidiConnectInputError")]
    MidiConnectInputError(#[from] midir::ConnectError<MidiInput>),
    #[error("MidiConnectOutputError")]
    MidiConnectOutputError(#[from] midir::ConnectError<MidiOutput>),
}
