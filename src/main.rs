mod backend;
mod settings;
mod ui;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Path to save the monitors.conf file")]
    set_monitors_path: Option<String>,

    #[arg(
        long,
        help = "Path to the main config.conf file to append the source line"
    )]
    set_config_path: Option<String>,

    #[arg(
        long,
        help = "Whether to auto-append 'source=./monitors.conf' to config.conf (true/false)"
    )]
    auto_append_source: Option<bool>,
}

fn main() -> iced::Result {
    let args = Args::parse();

    let mut exit_after_args = false;
    let mut app_settings = settings::AppSettings::load();

    if let Some(path) = args.set_monitors_path {
        app_settings.monitors_conf_path = path;
        exit_after_args = true;
    }
    if let Some(path) = args.set_config_path {
        app_settings.config_conf_path = path;
        exit_after_args = true;
    }
    if let Some(append) = args.auto_append_source {
        app_settings.auto_append_source = append;
        exit_after_args = true;
    }

    if exit_after_args {
        if let Err(e) = app_settings.save() {
            eprintln!("Error saving settings: {}", e);
            std::process::exit(1);
        }
        println!("Settings updated successfully.");
        return Ok(());
    }
    let custom_palette = iced::theme::Palette {
        background: iced::Color::from_rgb8(20, 20, 20),
        text: iced::Color::from_rgb8(230, 230, 230),
        primary: iced::Color::from_rgb8(100, 100, 100),
        success: iced::Color::from_rgb8(60, 60, 60),
        danger: iced::Color::from_rgb8(80, 80, 80),
        warning: iced::Color::from_rgb8(120, 120, 120),
    };

    let custom_theme = iced::Theme::Custom(std::sync::Arc::new(iced::theme::Custom::new(
        "MonoDark".to_string(),
        custom_palette,
    )));

    iced::application(
        ui::MangoDisplay::default,
        ui::MangoDisplay::update,
        ui::MangoDisplay::view,
    )
    .title("Mango Display")
    .theme(move |_app: &ui::MangoDisplay| custom_theme.clone())
    .window_size(iced::Size::new(1000.0, 700.0))
    .run()
}
