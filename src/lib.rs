// Copyright 2015 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[deny(missing_docs)]
extern crate gfx_core as core;
extern crate gfx_device_gl as device_gl;
extern crate glutin;

#[cfg(feature = "headless")]
pub use headless::{init_headless, init_headless_raw};

use core::memory::Typed;
use core::{format, handle, texture};
use device_gl::Resources as R;
use glutin::{CreationError, NotCurrent, PossiblyCurrent};

#[cfg(feature = "headless")]
mod headless;

/// Initialize with a window builder.
/// Generically parametrized version over the main framebuffer format.
///
/// # Example
///
/// ```no_run
/// extern crate gfx_core;
/// extern crate gfx_device_gl;
/// extern crate gfx_window_glutin;
/// extern crate glutin;
///
/// use gfx_core::format::{DepthStencil, Rgba8};
///
/// fn main() {
///     let event_loop = glutin::event_loop::EventLoop::new();
///     let window_builder = glutin::WindowBuilder::new().with_title("Example".to_string());
///     let context = glutin::ContextBuilder::new();
///     let (window, device, factory, rtv, stv) =
///         gfx_window_glutin::init::<Rgba8, DepthStencil>(window_builder, context, &event_loop)
///             .expect("Failed to create window");
///
///     // your code
/// }
/// ```
pub fn init<Cf, Df, T>(
    window: glutin::window::WindowBuilder,
    context: glutin::ContextBuilder<NotCurrent>,
    event_loop: &glutin::event_loop::EventLoop<T>,
) -> Result<
    (
        glutin::WindowedContext<PossiblyCurrent>,
        device_gl::Device,
        device_gl::Factory,
        handle::RenderTargetView<R, Cf>,
        handle::DepthStencilView<R, Df>,
    ),
    CreationError,
>
where
    Cf: format::RenderFormat,
    Df: format::DepthFormat,
{
    let (window, device, factory, color_view, ds_view) = init_raw(
        window,
        context,
        event_loop,
        Cf::get_format(),
        Df::get_format(),
    )?;

    Ok((
        window,
        device,
        factory,
        Typed::new(color_view),
        Typed::new(ds_view),
    ))
}

/// Initialize with an existing Glutin window.
/// Generically parametrized version over the main framebuffer format.
///
/// # Example (using Piston to create the window)
///
/// ```rust,ignore
/// extern crate piston;
/// extern crate glutin_window;
/// extern crate gfx_window_glutin;
///
/// // Create window with Piston
/// let settings = piston::window::WindowSettings::new("Example", [800, 600]);
/// let mut glutin_window = glutin_window::GlutinWindow::new(&settings).unwrap();
///
/// // Initialise gfx
/// let (mut device, mut factory, main_color, main_depth) =
///     gfx_window_glutin::init_existing::<ColorFormat, DepthFormat>(&glutin_window.window);
///
/// let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
/// ```
pub fn init_existing<Cf, Df>(
    window: glutin::WindowedContext<NotCurrent>,
) -> (
    glutin::WindowedContext<PossiblyCurrent>,
    device_gl::Device,
    device_gl::Factory,
    handle::RenderTargetView<R, Cf>,
    handle::DepthStencilView<R, Df>,
)
where
    Cf: format::RenderFormat,
    Df: format::DepthFormat,
{
    let (window, device, factory, color_view, ds_view) =
        init_existing_raw(window, Cf::get_format(), Df::get_format());
    (
        window,
        device,
        factory,
        Typed::new(color_view),
        Typed::new(ds_view),
    )
}

fn get_window_dimensions(ctx: &glutin::WindowedContext<PossiblyCurrent>) -> texture::Dimensions {
    let window = ctx.window();
    let (width, height) = {
        let size = window.inner_size().to_physical(window.hidpi_factor());
        (size.width as _, size.height as _)
    };
    let aa = ctx.get_pixel_format().multisampling.unwrap_or(0) as texture::NumSamples;

    (width, height, 1, aa.into())
}

/// Initialize with a window builder. Raw version.
pub fn init_raw<T>(
    window: glutin::window::WindowBuilder,
    context: glutin::ContextBuilder<NotCurrent>,
    event_loop: &glutin::event_loop::EventLoop<T>,
    color_format: format::Format,
    ds_format: format::Format,
) -> Result<
    (
        glutin::WindowedContext<PossiblyCurrent>,
        device_gl::Device,
        device_gl::Factory,
        handle::RawRenderTargetView<R>,
        handle::RawDepthStencilView<R>,
    ),
    CreationError,
> {
    let window = {
        let color_total_bits = color_format.0.get_total_bits();
        let alpha_bits = color_format.0.get_alpha_stencil_bits();
        let depth_total_bits = ds_format.0.get_total_bits();
        let stencil_bits = ds_format.0.get_alpha_stencil_bits();

        context
            .with_depth_buffer(depth_total_bits - stencil_bits)
            .with_stencil_buffer(stencil_bits)
            .with_pixel_format(color_total_bits - alpha_bits, alpha_bits)
            .with_srgb(color_format.1 == format::ChannelType::Srgb)
            .build_windowed(window, event_loop)?
    };

    let (window, device, factory, color_view, ds_view) =
        init_existing_raw(window, color_format, ds_format);

    Ok((window, device, factory, color_view, ds_view))
}

/// Initialize with an existing Glutin window. Raw version.
pub fn init_existing_raw(
    window: glutin::WindowedContext<NotCurrent>,
    color_format: format::Format,
    ds_format: format::Format,
) -> (
    glutin::WindowedContext<PossiblyCurrent>,
    device_gl::Device,
    device_gl::Factory,
    handle::RawRenderTargetView<R>,
    handle::RawDepthStencilView<R>,
) {
    let window = unsafe { window.make_current().unwrap() };
    let (device, factory) =
        device_gl::create(|s| window.get_proc_address(s) as *const std::os::raw::c_void);

    // create the main color/depth targets
    let dim = get_window_dimensions(&window);
    let (color_view, ds_view) =
        device_gl::create_main_targets_raw(dim, color_format.0, ds_format.0);

    // done
    (window, device, factory, color_view, ds_view)
}

/// Update the internal dimensions of the main framebuffer targets. Generic version over the format.
pub fn update_views<Cf, Df>(
    window: &glutin::WindowedContext<PossiblyCurrent>,
    color_view: &mut handle::RenderTargetView<R, Cf>,
    ds_view: &mut handle::DepthStencilView<R, Df>,
) where
    Cf: format::RenderFormat,
    Df: format::DepthFormat,
{
    let dim = color_view.get_dimensions();
    assert_eq!(dim, ds_view.get_dimensions());
    if let Some((cv, dv)) = update_views_raw(window, dim, Cf::get_format(), Df::get_format()) {
        *color_view = Typed::new(cv);
        *ds_view = Typed::new(dv);
    }
}

/// Return new main target views if the window resolution has changed from the old dimensions.
pub fn update_views_raw(
    window: &glutin::WindowedContext<PossiblyCurrent>,
    old_dimensions: texture::Dimensions,
    color_format: format::Format,
    ds_format: format::Format,
) -> Option<(
    handle::RawRenderTargetView<R>,
    handle::RawDepthStencilView<R>,
)> {
    let dim = get_window_dimensions(window);
    if dim != old_dimensions {
        Some(device_gl::create_main_targets_raw(
            dim,
            color_format.0,
            ds_format.0,
        ))
    } else {
        None
    }
}

/// Create new main target views based on the current size of the window.
/// Best called just after a WindowResize event.
pub fn new_views<Cf, Df>(
    window: &glutin::WindowedContext<PossiblyCurrent>,
) -> (
    handle::RenderTargetView<R, Cf>,
    handle::DepthStencilView<R, Df>,
)
where
    Cf: format::RenderFormat,
    Df: format::DepthFormat,
{
    let dim = get_window_dimensions(window);
    let (color_view_raw, depth_view_raw) =
        device_gl::create_main_targets_raw(dim, Cf::get_format().0, Df::get_format().0);
    (Typed::new(color_view_raw), Typed::new(depth_view_raw))
}
