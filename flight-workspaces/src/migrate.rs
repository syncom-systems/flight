// SPDX-License-Identifier: MIT

use crate::LoadError;

/// One schema upgrade, `steps[n]` taking version `n + 1` to `n + 2`. It edits the parsed table
/// and sets nothing else: the version number is advanced by [`run`].
pub type Step = fn(&mut toml::Table) -> Result<(), String>;

/// The upgrade path. A new schema version adds a step here and a test that loads the previous
/// version's file.
pub const STEPS: &[Step] = &[v1_to_v2];

/// Version 2 adds the optional `name` of a surface. Nothing to rewrite: a version 1 file is a
/// valid version 2 file, and the version number moves so that a build which does not know
/// `name` refuses the file instead of dropping the names.
fn v1_to_v2(_: &mut toml::Table) -> Result<(), String> {
    Ok(())
}

/// Bring `table` from `from` to the version `steps` leads to.
pub(crate) fn run(table: &mut toml::Table, from: u32, steps: &[Step]) -> Result<(), LoadError> {
    let mut version = from;
    for step in steps.iter().skip(from.saturating_sub(1) as usize) {
        step(table).map_err(LoadError::Migration)?;
        version = version.saturating_add(1);
        table.insert(
            "schema_version".to_owned(),
            toml::Value::Integer(i64::from(version)),
        );
    }
    Ok(())
}
