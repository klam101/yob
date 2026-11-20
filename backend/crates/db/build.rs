// Rebuild when migrations change: `sqlx::migrate!` embeds them at compile
// time, and cargo doesn't notice new .sql files on its own.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
