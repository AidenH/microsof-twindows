use std::process::Command;

use xcb::{x, x::{EventMask, ConfigureRequestEvent}};

fn add_window(con: &xcb::Connection, e: ConfigureRequestEvent) -> xcb::Result<()> {
    con.send_request(&x::MapWindow {
        window: e.window(),
    });
    con.flush()?;
    Ok(())
}

fn main() -> xcb::Result<()> {
    let (con, scr_num) = xcb::Connection::connect(None).unwrap();
    let setup = con.get_setup();
    let scr = setup.roots().nth(scr_num as usize).unwrap();

    let cookie = con.send_request_checked(&x::ChangeWindowAttributes {
        window: scr.root(),
        value_list: &[
            x::Cw::BackPixel(scr.black_pixel()),
            x::Cw::EventMask(EventMask::KEY_PRESS | EventMask:: SUBSTRUCTURE_REDIRECT),
        ]
    });

    con.check_request(cookie)?;

    loop {
        match con.wait_for_event()? {
            xcb::Event::X(x::Event::KeyPress(e)) => {
                println!("{}", e.detail());

                if e.detail() == 58 { // 'm'
                    break Ok(());
                } else if e.detail() == 38 { // 'a'
                    Command::new("arandr");
                }
            }

            xcb::Event::X(x::Event::ConfigureRequest(e)) => {
                println!("{:?}", e);
                add_window(&con, e)?;
            }

            _ => {}
        }
    }
}
