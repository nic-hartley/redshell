//! This module provides input adapters for each of the various I/O mechanisms supported. Each one is controlled by
//! a feature named similarly and exports a struct implementing `IoSystem`. The actual intended input and output APIs
//! are in the `input` and `output` modules.

#[cfg(feature = "__sys")]
use std::collections::HashMap;
use std::{io, sync::{Arc, Barrier}, task::Poll};

use futures::poll;

use super::{input::Action, output::Screen, XY};

#[cfg(feature = "sys_cli")]
pub mod ansi_cli;

#[cfg(feature = "__sys_gui")]
pub mod gui;

/// An input/output system.
/// 
/// The output is called a "display" to distinguish it from the [`Screen`].
/// 
/// This object is meant to be associated with a [`IoRunner`], which will run infinitely on the main thread while this
/// is called from within the async context.
#[async_trait::async_trait]
pub trait IoSystem: Send {
    /// Actually render a [`Screen`] to the display.
    async fn draw(&mut self, screen: &Screen) -> io::Result<()>;
    /// Get the size of the display, in characters.
    fn size(&self) -> XY;
    /// Asyncly wait for the next user input.
    async fn input(&mut self) -> io::Result<Action>;
    /// Tells the associated [`IoRunner`] to stop and return control of the main thread, and tell the [`IoSystem`] to
    /// dispose of any resources it's handling.
    /// 
    /// This will always be the last method called on this object (unless you count `Drop::drop`) so feel free to
    /// panic in the others if they're called after this one, especially `draw`.
    fn stop(&mut self);

    /// Clear out queued events without processing them at all. Returns the last action found, or the first error.
    /// Returns `None` if there was nothing queued.
    async fn flush(&mut self) -> io::Result<Option<Action>> {
        let mut last = None;
        while let Poll::Ready(act) = poll!(self.input()) {
            // raise up errors if they occur
            last = Some(act?);
            // otherwise nothing else needs to be done
        }
        Ok(last)
    }
}

/// The other half of an [`IoSystem`].
/// 
/// This type exists so that things which need to run on the main thread specifically, can.
pub trait IoRunner {
    /// Run until the paired [`IoSystem`] tells you to stop.
    fn run(&mut self);
}

/// An implementation of [`IoRunner`] for backends which don't actually require anything in particular be done on the
/// main thread.
/// 
/// The intended use of this is creating one, returning its clone, and telling your copy to stop when the [`IoSystem`]
/// method is called.
#[derive(Clone)]
pub struct NopIoRunner(Arc<Barrier>);

impl NopIoRunner {
    /// Create a [`NopIoRunner`].
    pub fn new() -> Self {
        Self(Arc::new(Barrier::new(2)))
    }

    /// Tell the [`NopIoRunner`] to stop.
    pub fn stop(&mut self) {
        self.0.wait();
    }
}

impl IoRunner for NopIoRunner {
    fn run(&mut self) {
        self.0.wait();
    }
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
pub fn load() -> Result<(Box<dyn IoSystem>, Box<dyn IoRunner>), HashMap<&'static str, io::Error>> {
    let mut errors = HashMap::new();
    macro_rules! try_init {
        ( $name:ident: $( $init:tt )* ) => {
            let res = {
                $($init)*
            };
            match res {
                Ok((iosys, run)) => return Ok((Box::new(iosys), Box::new(run))),
                Err(e) => errors.insert(stringify!($name), e),
            };
        }
    }
    #[cfg(feature = "__sys_gui")]
    {
        use crate::io::sys::gui::Gui;
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
            use crate::io::sys::gui::softbuffer::SoftbufferBackend;
            // Try to initialize softbuffer rendering
            try_init! { softbuffer_gui: Gui::<SoftbufferBackend>::new(20.0) }
        }
    }
    #[cfg(feature = "sys_cli")]
    {
        // Try to initialize the CLI renderer
        try_init! { ansi_cli: ansi_cli::AnsiIo::get() }
    }
    Err(errors)
}
