use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::group::Group;
use super::record::Record;

/// Represents a plugin file
///
/// This type can be used to read and write plugin (mod) files. These are .esm files (masters, on
/// which other plugins may depend) and .esp files (regular plugin files). Note that the
/// `is_master` flag determines whether a plugin is a master, not the file extension; using .esm for
/// masters and .esp for non-masters is merely convention.
///
/// Plugins consist of a series of records which represent all the different objects in the game
/// world. Each record consists of one or more fields containing the data and attributes of the
/// object.
///
/// Plugins can be read from a file with [`load_file`] and saved to a file with [`save_file`]. You
/// can also use [`read`] and [`write`] directly to read a plugin from/write a plugin to a buffer
/// or any other type implementing [`Read`] or [`Write`], respectively. A new, empty plugin may be
/// created with [`new`].
///
/// [`load_file`]: #method.load_file
/// [`save_file`]: #method.save_file
/// [`read`]: #method.read
/// [`write`]: #method.write
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`new`]: #method.new
#[derive(Debug)]
pub struct Plugin {
    version: f32,
    /// Indicates whether this plugin is a master
    pub is_master: bool,
    next_form_id: u32,
    author: String,
    description: String,
    masters: Vec<String>,
    groups: HashMap<[u8; 4], Group>,
    id_map: HashMap<u32, Rc<RefCell<Record>>>,
}

/// Version value for Oblivion plugins
pub const VERSION: f32 = 1.;

impl Plugin {
    pub fn new(author: String, description: String) -> Plugin {
        Plugin {
            version: VERSION,
            is_master: false,
            next_form_id: 1,
            author,
            description,
            masters: vec![],
            groups: HashMap::new(),
            id_map: HashMap::new(),
        }
    }
}