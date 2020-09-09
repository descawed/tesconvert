use std::fs;
use std::path::Path;

use crate::*;

/// The full set of objects in the game world
///
/// The World type manages the current load order of plugins and allows looking up records from
/// the appropriate plugin based on load order.
pub trait WorldInterface {
    type Plugin: PluginInterface;

    fn load_plugins<P, S, T>(
        plugin_dir: P,
        plugin_names: T,
    ) -> Result<Vec<(String, Self::Plugin)>, TesError>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
        T: Iterator<Item = S>,
    {
        let plugin_path = plugin_dir.as_ref();
        let mut files = vec![];
        for filename in plugin_names {
            let plugin_path = plugin_path.join(filename.as_ref());
            let meta = fs::metadata(&plugin_path)?;
            files.push((plugin_path, meta.modified()?));
        }

        files.sort_by(|(_, a), (_, b)| a.cmp(&b));

        let mut plugins = Vec::with_capacity(files.len());
        for (path, _) in files {
            let plugin = Self::Plugin::load_file(&path)?;
            // The file_name() unwrap should be safe because if this wasn't a path that pointed to a
            // file, Plugin::load_file would have failed. The into_string() unwrap should be safe
            // because I got these filenames from an iterator of strings to begin with, so the reverse
            // conversion should always be possible.
            plugins.push((
                path.file_name()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap(),
                plugin,
            ));
        }

        Ok(plugins)
    }
}
