//! Xlib calls related to a mouse.
use x11rb::{protocol::xproto, x11_utils::Serialize};

use super::{button_event_mask, mouse_event_mask, XWrap};

impl XWrap {
    /// Grabs the mouse clicks of a window.
    pub fn grab_mouse_clicks(&self, handle: xproto::Window, is_focused: bool) {
        self.ungrab_buttons(handle);
        if !is_focused {
            self.grab_buttons(handle, xproto::ButtonIndex::M1, xproto::ModMask::ANY);
            self.grab_buttons(handle, xproto::ButtonIndex::M3, xproto::ModMask::ANY);
        }
        let mouse_key_mask = xproto::ModMask::from(self.mouse_key_mask as u16);
        self.grab_buttons(handle, xproto::ButtonIndex::M1, mouse_key_mask);
        self.grab_buttons(
            handle,
            xproto::ButtonIndex::M1,
            mouse_key_mask | xproto::ModMask::SHIFT,
        );
        self.grab_buttons(handle, xproto::ButtonIndex::M3, mouse_key_mask);
        self.grab_buttons(
            handle,
            xproto::ButtonIndex::M3,
            mouse_key_mask | xproto::ModMask::SHIFT,
        );
    }

    /// Grabs the button with the modifier for a window.
    // `XGrabButton`: https://tronche.com/gui/x/xlib/input/XGrabButton.html
    pub fn grab_buttons(
        &self,
        window: xproto::Window,
        button: xproto::ButtonIndex,
        modifiers: xproto::ModMask,
    ) {
        let mods: Vec<xproto::ModMask> = vec![
            modifiers,
            modifiers | xproto::ModMask::M2,
            modifiers | xproto::ModMask::LOCK,
        ];
        for m in mods {
            xproto::grab_button(
                &self.conn,
                false,
                window,
                button_event_mask(),
                xproto::GrabMode::ASYNC,
                xproto::GrabMode::ASYNC,
                x11rb::NONE,
                x11rb::NONE,
                button,
                m,
            )
            .unwrap();
        }
    }

    /// Cleans all currently grabbed buttons of a window.
    // `XUngrabButton`: https://tronche.com/gui/x/xlib/input/XUngrabButton.html
    pub fn ungrab_buttons(&self, handle: xproto::Window) {
        xproto::ungrab_button(
            &self.conn,
            xproto::ButtonIndex::ANY,
            handle,
            xproto::ModMask::ANY,
        )
        .unwrap();
    }

    /// Grabs the cursor and sets its visual.
    pub fn grab_pointer(&self, cursor: xproto::Cursor) {
        xproto::grab_pointer(
            &self.conn,
            false,
            self.root,
            mouse_event_mask(),
            xproto::GrabMode::ASYNC,
            xproto::GrabMode::ASYNC,
            x11rb::NONE,
            cursor,
            x11rb::CURRENT_TIME,
        )
        .unwrap();
    }

    /// Ungrab the cursor.
    // `XUngrabPointer`: https://tronche.com/gui/x/xlib/input/XUngrabPointer.html
    pub fn ungrab_pointer(&self) {
        xproto::ungrab_pointer(&self.conn, x11rb::CURRENT_TIME).unwrap();
    }

    /// Move the cursor to a window.
    /// # Errors
    ///
    /// Will error if unable to obtain window attributes. See `get_window_attrs`.
    pub fn move_cursor_to_window(&self, window: xproto::Window) -> Option<()> {
        let size_hints = self.get_hint_sizing(window)?;
        let (_, x, y) = size_hints.position?;
        let (_, width, height) = size_hints.size?;
        let point = (x + (width / 2), y + (height / 2));
        self.move_cursor_to_point(point).ok()
    }

    /// Move the cursor to a point.
    /// # Errors
    ///
    /// Error indicates `XlibError`.
    // `XWarpPointer`: https://tronche.com/gui/x/xlib/input/XWarpPointer.html
    // TODO: Verify that Error is unreachable or specify conditions that may result
    // in an error.
    pub fn move_cursor_to_point(
        &self,
        point: (i32, i32),
    ) -> Result<(), Box<dyn std::error::Error>> {
        if point.0 >= 0 && point.1 >= 0 {
            xproto::warp_pointer(
                &self.conn,
                x11rb::NONE,
                self.root,
                x11rb::NONE as i16,
                x11rb::NONE as i16,
                x11rb::NONE as u16,
                x11rb::NONE as u16,
                point.0 as i16,
                point.1 as i16,
            )?;
        }
        Ok(())
    }

    /// Replay a click on a window.
    // `XQueryPointer`: https://tronche.com/gui/x/xlib/window-information/XQueryPointer.html
    pub fn replay_click(&self, focused_window: xproto::Window, button: xproto::Button) {
        let mut event = xproto::ButtonPressEvent {
            child: self.get_default_root(),
            detail: button,
            same_screen: true,
            ..Default::default()
        };
        event.child = self.get_default_root();
        loop {
            let reply = xproto::query_pointer(&self.conn, event.child)
                .unwrap()
                .reply()
                .unwrap();
            if reply.child != x11rb::NONE {
                break;
            }
            event.child = reply.child;
            event.root = reply.root;
            event.root_x = reply.root_x;
            event.root_y = reply.root_y;
            event.event_x = reply.win_x;
            event.event_y = reply.win_y;
        }

        if event.child == focused_window {
            event.state = xproto::KeyButMask::BUTTON1;
            self.send_xevent(
                event.child,
                false,
                xproto::EventMask::BUTTON_PRESS,
                &event.serialize(),
            );

            event.state.remove(xproto::KeyButMask::BUTTON1);
            self.send_xevent(
                event.child,
                false,
                xproto::EventMask::BUTTON_RELEASE,
                &event.serialize(),
            );
        }
    }
}
