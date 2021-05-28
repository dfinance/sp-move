use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

const MOVE_STDLIB_REPO_URL: &str = "https://github.com/pontem-network/move-stdlib/";
const MOVE_STDLIB_REPO_REVISION: &str = "master";

fn main() -> anyhow::Result<()> {
    generate_cargo_keys();

    rerun_if_git_head_changed();

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let move_stdlib_dir = out_dir.join("move_stdlib");
    if !move_stdlib_dir.exists() {
        let status = std::process::Command::new("git").arg("clone").arg(MOVE_STDLIB_REPO_URL).arg(&move_stdlib_dir).status()?;
        anyhow::ensure!(status.success(), "Running `git clone {} {}` failed with code {}", MOVE_STDLIB_REPO_URL, move_stdlib_dir.display(), status);
        let status = std::process::Command::new("git").current_dir(&move_stdlib_dir).arg("checkout").arg(MOVE_STDLIB_REPO_REVISION).status()?;
        anyhow::ensure!(status.success(), "Running `git checkout {}` failed with code {}", MOVE_STDLIB_REPO_REVISION, status);
        let status = std::process::Command::new("dove").current_dir(move_stdlib_dir).arg("build").arg("--package").status()?;
        anyhow::ensure!(status.success(), "Running `dove build` failed with code {}", status);
    }

    Ok(())
}
