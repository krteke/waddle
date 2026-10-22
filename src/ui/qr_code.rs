use adw::prelude::*;
use qrcode::{Color, types::QrError};
use relm4::{adw, gtk};

pub struct QrCode {
    pub widget: adw::Bin,
}

impl QrCode {
    pub fn new(text: &str, label: &str) -> Result<Self, QrError> {
        let code = qrcode::QrCode::new(text.as_bytes())?;

        let modules = code.width() + 8;
        let size = (220 / modules * modules) as i32;

        let area = gtk::DrawingArea::builder()
            .content_width(size)
            .content_height(size)
            .accessible_role(gtk::AccessibleRole::Img)
            .build();

        area.update_property(&[gtk::accessible::Property::Label(label)]);

        area.set_draw_func(move |_, context, width, height| {
            context.set_source_rgb(1.0, 1.0, 1.0);
            context.paint().expect("QR code background drawing failed");

            let side = width.min(height) as f64;
            let scale = side / modules as f64;

            context.translate((width as f64 - side) / 2.0, (height as f64 - side) / 2.0);
            context.scale(scale, scale);
            context.set_antialias(gtk::cairo::Antialias::None);
            context.set_source_rgb(0.0, 0.0, 0.0);

            for y in 0..code.width() {
                for x in 0..code.width() {
                    if code[(x, y)] == Color::Dark {
                        // Four empty modules on each side form the QR quiet zone.
                        context.rectangle((x + 4) as f64, (y + 4) as f64, 1.0, 1.0);
                    }
                }
            }

            context.fill().expect("QR code module drawing failed");
        });

        let widget = adw::Bin::builder()
            .halign(gtk::Align::Center)
            .css_classes(["card"])
            .overflow(gtk::Overflow::Hidden)
            .child(&area)
            .build();

        Ok(Self { widget })
    }
}
