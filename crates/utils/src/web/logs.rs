
use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{fmt::MakeWriter, Registry};
use wasm_bindgen::prelude::*;

#[derive(Resource, Clone, Default)]
pub struct WebLogBuffer(pub Arc<Mutex<String>>);

#[derive(Clone)]
pub struct WebLogWriter {
    buffer: Arc<Mutex<String>>,
}

impl std::io::Write for WebLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut buffer_guard) = self.buffer.lock() {
            if let Ok(s) = std::str::from_utf8(buf) {
                buffer_guard.push_str(s);

                const MAX_LOG_BYTES: usize = 5 * 1024 * 1024; // Limite à 5 Mo
                if buffer_guard.len() > MAX_LOG_BYTES {
                    let to_remove = buffer_guard.len() - MAX_LOG_BYTES;
                    buffer_guard.drain(..to_remove);
                }
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for WebLogBuffer {
    type Writer = WebLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        WebLogWriter { buffer: self.0.clone() }
    }
}

// ---- 2. Interopérabilité WASM pour le Téléchargement ----

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = download_log_file_js)] // Correspond au nom de la fonction JS
    fn download_log_file_js_binding(filename: &str, content: &str);
}

// Fonction Rust appelable pour initier le téléchargement via JS
fn initiate_log_download_from_rust(logs_content: String, filename: &str) {
    if cfg!(target_arch = "wasm32") { // S'assure que cela n'est appelé que sur WASM
        unsafe {
            download_log_file_js_binding(filename, &logs_content);
        }
    } else {
        warn!("Tentative de téléchargement de log sur une plateforme non-WASM. Ignoré.");
        // Pour le débogage natif, vous pourriez écrire dans un fichier local ici.
        // std::fs::write(filename, logs_content).expect("Impossible d'écrire le log localement");
        // log::info!("Log sauvegardé localement (natif) dans : {}", filename);
    }
}

// ---- 3. Systèmes Bevy pour Déclencher le Téléchargement ----

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DownloadLogTrigger(pub bool);

// Système qui écoute une entrée (ex: touche 'L')
pub fn check_for_log_download_request(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut trigger: ResMut<DownloadLogTrigger>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        if !trigger.0 {
            info!("Demande de téléchargement du log reçue (touche L)!");
            trigger.0 = true;
        }
    }
}

// Système qui traite la demande et lance le téléchargement
pub fn process_log_download_trigger(
    mut trigger: ResMut<DownloadLogTrigger>,
    log_buffer: Res<WebLogBuffer>,
    frame: Res<crate::frame::FrameCount>, // Assurez-vous que votre ressource FrameCount est accessible
) {
    if trigger.0 {
        let logs_string;
        {
            let buffer_guard = log_buffer.0.lock().unwrap();
            logs_string = buffer_guard.clone();
        }

        if !logs_string.is_empty() {
            let filename = format!("game_log_frame_{}.txt", frame.frame); // Utilisez le numéro de frame GGRS ici si possible
            initiate_log_download_from_rust(logs_string, &filename);
        } else {
            warn!("Tentative de téléchargement d'un buffer de log vide.");
        }
        
        trigger.0 = false;
    }
}

// ---- 4. Plugin Bevy pour la Configuration ----

// Système de démarrage pour configurer l'abonné tracing
fn setup_wasm_logging_subscriber_system(mut commands: Commands) {
    let log_buffer = WebLogBuffer::default();
    commands.insert_resource(log_buffer.clone());

    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,votre_crate=debug")) // Ajustez "votre_crate" avec le nom de votre crate principal
        .unwrap();

    // Format pour les logs (sans horodatage, sans couleurs ANSI pour le buffer)
    let log_format = fmt::format()
        .without_time()
        .with_ansi(false) // Pas de couleurs pour le buffer qui deviendra un fichier
        .compact(); // ou .pretty() selon la préférence

    let web_log_layer = fmt::Layer::new()
        .event_format(log_format)
        .with_writer(log_buffer); // WebLogBuffer implémente MakeWriter

    // Optionnel : pour voir les logs dans la console du navigateur également
    // Nécessite la crate `tracing-web` et la feature "wasm-bindgen" pour celle-ci.
    // let console_layer = tracing_web:: برخی از لایه ها برای کنسول وب;


    let subscriber = Registry::default()
        .with(filter)
        .with(web_log_layer);
        // .with(console_layer); // Si vous l'ajoutez

    tracing::subscriber::set_global_default(subscriber)
        .expect("Impossible de définir l'abonné tracing global");

    info!("Journalisation WASM initialisée. Logs bufferisés en mémoire. Appuyez sur 'L' pour télécharger.");
}



pub struct WasmWebLogPlugin;

impl Plugin for WasmWebLogPlugin {
    fn build(&self, app: &mut App) {
        // S'assure que le plugin n'est ajouté que pour la cible WASM
        if cfg!(target_arch = "wasm32") {
            app.add_systems(Startup, setup_wasm_logging_subscriber_system)
               .init_resource::<DownloadLogTrigger>()
               .add_systems(Update, (
                    check_for_log_download_request,
                    process_log_download_trigger,
                ).chain()); // .chain() assure l'ordre si nécessaire
        }
    }
}
