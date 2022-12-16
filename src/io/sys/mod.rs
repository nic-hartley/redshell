//! This module provides input adapters for each of the various I/O mechanisms supported. Each one is controlled by
//! a feature named similarly and exports a struct implementing `IoSystem`. The actual intended input and output APIs
//! are in the `input` and `output` modules.

#[cfg(feature = "__sys")]
use std::collections::HashMap;
use std::io;

use super::{input::Action, output::Screen, XY};

#[cfg(feature = "sys_cli")]
pub mod ansi_cli;

#[cfg(feature = "__sys_gui")]
pub mod gui;

#[async_trait::async_trait]
pub trait IoSystem {
    async fn draw(&mut self, screen: &Screen) -> io::Result<()>;
    fn size(&self) -> XY;
    async fn input(&mut self) -> io::Result<Action>;
}

/// Based on IO system features enabled, attempt to initialize an IO system; in order:
///
/// - Vulkan GUI (`gui_vulkan`)
/// - OpenGL GUI (`gui_opengl`)
/// - CPU-rendered GUI (`gui_cpu`)
/// - crossterm CLI (`cli_crossterm`)
///
/// The Err type is a map from the name of the system (in code formatting above) to the error that it hit.
#[cfg(feature = "__sys")]
pub async fn load() -> Result<Box<dyn IoSystem>, HashMap<&'static str, io::Error>> {
    let mut errors = HashMap::new();
    macro_rules! try_init {
        ( $name:ident: $( $init:tt )* ) => {
            let res = || {
                $($init)*
            };
            match res() {
                Ok(res) => return Ok(Box::new(res)),
                Err(e) => errors.insert(stringify!($name), e),
            };
        }
    }
    #[cfg(feature = "__sys_gui")]
    {
        #[cfg(feature = "sys_gui_vulkan")]
        {
            // TODO: Try to initialize Vulkan rendering
        }
        #[cfg(feature = "sys_gui_opengl")]
        {
            // TODO: Try to initialize OpenGL rendering
        }
        #[cfg(feature = "sys_gui_softbuffer")]
        {
            // TODO: Try to initialize softbuffer rendering
        }
    }
    #[cfg(feature = "sys_cli")]
    {
        // Try to initialize the CLI renderer
        try_init! { ansi_cli: ansi_cli::AnsiIo::get() }
    }
    Err(errors)
}
