slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    // // Callback wird jedes Mal ausgelöst, wenn ein Slider bewegt wird
    // app.on_color_changed(|selected_color| {
    //     println!("Farbe im Backend aktualisiert: {:?}", selected_color);
    //     // Hier kannst du die Farbe verarbeiten
    // });
    let app_weak = app.as_weak();

    // Wird aufgerufen, wenn "Accept" gedrückt wurde
    app.on_final_color_changed(move |_| {
        if let Some(app) = app_weak.upgrade() {
            // Farbwerte aus UI holen
            let r = (app.get_current_r() * 255.0).round() as u8;
            let g = (app.get_current_g() * 255.0).round() as u8;
            let b = (app.get_b() * 255.0).round() as u8;

            // Hex-String in Rust generieren
            let hex_string = format!("#{:02X}{:02X}{:02X}", r, g, b);
            println!("Farbe erfolgreich in Rust übernommen: {}", hex_string);

            // Hex-String zurück an die UI senden
            app.set_hex_display(hex_string.into());
        }
    });
    app.run()
}
