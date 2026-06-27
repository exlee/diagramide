#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "yaml"

# ── Configuration ──────────────────────────────────────────────────────────────

OUTPUT                  = File.join(__dir__, "nightly_v2.yml")
CHECKOUT_ACTION         = "actions/checkout@v5"
CACHE_ACTION            = "actions/cache@v5"
UPLOAD_ARTIFACT_ACTION  = "actions/upload-artifact@v7"
DOWNLOAD_ARTIFACT_ACTION = "actions/download-artifact@v7"
RUST_CACHE_ACTION       = "Swatinem/rust-cache@v2"
RELEASE_ACTION          = "softprops/action-gh-release@v3"
ZIG_VERSION             = "0.13.0"

TARGETS = [
  { id: "windows_x86_64",  target: "x86_64-pc-windows-gnu",      os: "ubuntu-latest", kind: :windows },
  { id: "windows_aarch64", target: "aarch64-pc-windows-gnullvm", os: "ubuntu-latest", kind: :windows },
  { id: "linux_x86_64",    target: "x86_64-unknown-linux-musl",  os: "ubuntu-latest", kind: :zig     },
  { id: "linux_aarch64",   target: "aarch64-unknown-linux-musl", os: "ubuntu-latest", kind: :zig     },
  { id: "macos_x86_64",    target: "x86_64-apple-darwin",        os: "macos-latest",  kind: :macos   },
  { id: "macos_aarch64",   target: "aarch64-apple-darwin",       os: "macos-latest",  kind: :macos   }
].freeze

# macOS targets fan out into two lanes (DMG + raw binaries); everything else is one lane.
LANES = TARGETS.flat_map do |target|
  if target[:kind] == :macos
    [
      target.merge(lane: :dmg,    lane_id: "#{target[:id]}_dmg"),
      target.merge(lane: :binary, lane_id: "#{target[:id]}_binary")
    ]
  else
    [target.merge(lane: :binaries, lane_id: target[:id])]
  end
end.freeze

# Paths persisted by the shared toolchain/cache restore.
CACHED_PATHS = %w[
  ~/.local/zig
  ~/.cargo/bin
  ~/.cargo/git
  ~/.cargo/registry
  ~/.rustup/settings.toml
  ~/.rustup/toolchains
  ~/.rustup/update-hashes
].freeze

# ── Helpers ────────────────────────────────────────────────────────────────────

def command(*lines)
  lines.join("\n")
end

def install_cargo_tool(name, install_command)
  {
    "name" => "Install #{name}",
    "run" => command(
      "if ! command -v #{name} &> /dev/null; then",
      "  #{install_command}",
      "fi"
    )
  }
end

# ── Step builders ──────────────────────────────────────────────────────────────

def checkout_step
  { "uses" => CHECKOUT_ACTION }
end

def cache_step(target)
  {
    "name" => "Restore Toolchain and Cargo Cache",
    "uses" => CACHE_ACTION,
    "with" => {
      "path" => CACHED_PATHS.join("\n"),
      "key" => "${{ runner.os }}-#{target}-prepare-v1-${{ hashFiles('Cargo.lock') }}",
      "restore-keys" => "${{ runner.os }}-#{target}-prepare-v1-"
    }
  }
end

def rust_toolchain_step(target)
  {
    "name" => "Install Rust Toolchain",
    "run" => command("rustup update stable", "rustup target add #{target}")
  }
end

def cargo_fetch_step
  { "name" => "Fetch Cargo Dependencies", "run" => "cargo fetch --locked" }
end

def install_zig_step
  {
    "name" => "Install Zig",
    "shell" => "bash",
    "run" => command(
      "set -euo pipefail",
      "zig_dir=\"$HOME/.local/zig/#{ZIG_VERSION}\"",
      "if [ ! -x \"$zig_dir/zig\" ]; then",
      "  rm -rf \"$zig_dir\"",
      "  mkdir -p \"$zig_dir\"",
      "  curl -L \"https://ziglang.org/download/#{ZIG_VERSION}/zig-linux-$(uname -m)-#{ZIG_VERSION}.tar.xz\" -o /tmp/zig.tar.xz",
      "  tar -xJf /tmp/zig.tar.xz --strip-components=1 -C \"$zig_dir\"",
      "fi",
      "echo \"$zig_dir\" >> \"$GITHUB_PATH\""
    )
  }
end

def install_mingw_step
  {
    "name" => "Install MinGW-w64 (Required for Windows GNU targets)",
    "run" => command("sudo apt-get update", "sudo apt-get install -y mingw-w64")
  }
end

def install_cargo_zigbuild_step
  install_cargo_tool("cargo-zigbuild", "cargo install cargo-zigbuild")
end

def install_cargo_bundle_step
  install_cargo_tool("cargo-bundle", "cargo install cargo-bundle --version 0.11.0 --locked")
end

def tool_steps_for(kind)
  kind == :macos ? [install_cargo_bundle_step] : [install_cargo_zigbuild_step]
end

def build_cache_step(target)
  {
    "name" => "Restore Rust Build Cache",
    "uses" => RUST_CACHE_ACTION,
    "with" => { "key" => target, "cache-on-failure" => true }
  }
end

def compile_step(target)
  { "name" => "Compile", "run" => "cargo zigbuild --target #{target} --release" }
end

def compile_macos_binary_step(target)
  {
    "name" => "Compile Standalone Binaries",
    "run" => "cargo build --release --target #{target} --package pikchr_pl --package pikchr_pro --package diagramide"
  }
end

def bundle_dmg_step(target)
  {
    "name" => "Bundle DiagramIDE DMG",
    "run" => command(
      "cargo bundle --release --package diagramide --target #{target} --format dmg",
      "mv target/#{target}/release/bundle/dmg/DiagramIDE*.dmg \\",
      "  target/#{target}/release/#{target}-DiagramIDE.dmg"
    )
  }
end

def compile_action_step(lane)
  case lane[:lane]
  when :dmg    then bundle_dmg_step(lane[:target])
  when :binary then compile_macos_binary_step(lane[:target])
  else              compile_step(lane[:target])
  end
end

def collect_raw_artifacts_step(lane)
  target  = lane[:target]
  raw_dir = "dist/raw/#{lane[:lane_id]}"

  {
    "name" => "Collect Raw Artifacts",
    "shell" => "bash",
    "run" => case lane[:lane]
             when :dmg
               command(
                 "mkdir -p #{raw_dir}",
                 "cp target/#{target}/release/#{target}-DiagramIDE.dmg #{raw_dir}/"
               )
             when :binary
               command(
                 "mkdir -p #{raw_dir}",
                 "cp target/#{target}/release/pikchr_pl #{raw_dir}/",
                 "cp target/#{target}/release/pikchr_pro #{raw_dir}/",
                 "cp target/#{target}/release/diagramide #{raw_dir}/"
               )
             else
               command(
                 "mkdir -p #{raw_dir}",
                 "cp target/#{target}/release/pikchr_pl* #{raw_dir}/",
                 "cp target/#{target}/release/pikchr_pro* #{raw_dir}/",
                 "cp target/#{target}/release/diagramide* #{raw_dir}/"
               )
             end
  }
end

def upload_raw_artifact_step(lane)
  lane_id = lane[:lane_id]
  {
    "name" => "Upload Raw Artifact",
    "uses" => UPLOAD_ARTIFACT_ACTION,
    "with" => {
      "name" => "raw-#{lane_id}",
      "path" => "dist/raw/#{lane_id}/*",
      "retention-days" => 1
    }
  }
end

def download_raw_artifact_step(lane)
  lane_id = lane[:lane_id]
  {
    "name" => "Download Raw Artifact",
    "uses" => DOWNLOAD_ARTIFACT_ACTION,
    "with" => { "name" => "raw-#{lane_id}", "path" => "dist/raw/#{lane_id}" }
  }
end

def package_artifacts_step(lane)
  target  = lane[:target]
  lane_id = lane[:lane_id]
  raw     = "dist/raw/#{lane_id}"
  final   = "dist/final/#{lane_id}"

  {
    "name" => "Package Artifacts",
    "shell" => "bash",
    "run" => case lane[:lane]
             when :dmg
               command(
                 "mkdir -p #{final}",
                 "cp #{raw}/#{target}-DiagramIDE.dmg #{final}/"
               )
             when :binary
               command(
                 "mkdir -p #{final}",
                 "mv #{raw}/pikchr_pl \\",
                 "  #{final}/#{target}-pikchr.pl",
                 "mv #{raw}/pikchr_pro \\",
                 "  #{final}/#{target}-pikchr.pro",
                 "mv #{raw}/diagramide \\",
                 "  #{final}/#{target}-diagramide"
               )
             else
               command(
                 "mkdir -p #{final}",
                 "cd #{raw}",
                 "",
                 "if [ -f \"pikchr_pl.exe\" ]; then",
                 "  mv pikchr_pl.exe  ../../final/#{lane_id}/#{target}-pikchr.pl.exe",
                 "  mv pikchr_pro.exe ../../final/#{lane_id}/#{target}-pikchr.pro.exe",
                 "  mv diagramide.exe ../../final/#{lane_id}/#{target}-diagramide.exe",
                 "else",
                 "  mv pikchr_pl  ../../final/#{lane_id}/#{target}-pikchr.pl",
                 "  mv pikchr_pro ../../final/#{lane_id}/#{target}-pikchr.pro",
                 "  mv diagramide ../../final/#{lane_id}/#{target}-diagramide",
                 "fi"
               )
             end
  }
end

def final_artifact_name(lane)
  case lane[:lane]
  when :dmg    then "binaries-#{lane[:target]}-dmg"
  when :binary then "binaries-#{lane[:target]}-binary"
  else              "binaries-#{lane[:target]}"
  end
end

def upload_final_artifact_step(lane)
  target  = lane[:target]
  lane_id = lane[:lane_id]
  final   = "dist/final/#{lane_id}"

  path = case lane[:lane]
         when :dmg
           "#{final}/#{target}-DiagramIDE.dmg"
         when :binary
           [
             "#{final}/#{target}-pikchr.pl",
             "#{final}/#{target}-pikchr.pro",
             "#{final}/#{target}-diagramide"
           ].join("\n")
         else
           [
             "#{final}/#{target}-pikchr.pl*",
             "#{final}/#{target}-pikchr.pro*",
             "#{final}/#{target}-diagramide*",
             "!**/*.d",
             "!**/*.rlib"
           ].join("\n")
         end

  {
    "name" => "Upload Final Artifact",
    "uses" => UPLOAD_ARTIFACT_ACTION,
    "with" => { "name" => final_artifact_name(lane), "path" => path }
  }
end

# ── Job assembly ───────────────────────────────────────────────────────────────

def prepare_steps(target)
  [
    checkout_step,
    cache_step(target[:target]),
    rust_toolchain_step(target[:target]),
    cargo_fetch_step,
    *(target[:kind] == :macos ? [] : [install_zig_step]),
    *tool_steps_for(target[:kind])
  ]
end

def prepare_job(target)
  {
    "name" => "Prepare #{target[:target]}",
    "runs-on" => target[:os],
    "steps" => prepare_steps(target)
  }
end

def compile_steps(lane)
  macos  = lane[:kind] == :macos
  target = lane[:target]

  steps = [checkout_step]
  steps << install_mingw_step if lane[:kind] == :windows
  steps << cache_step(target)
  steps << install_zig_step unless macos
  steps << rust_toolchain_step(target)
  steps.concat(compile_tool_steps(lane))
  steps << build_cache_step(target)
  steps << compile_action_step(lane)
  steps << collect_raw_artifacts_step(lane)
  steps << upload_raw_artifact_step(lane)
end

def compile_tool_steps(lane)
  case lane[:lane]
  when :dmg    then [install_cargo_bundle_step]
  when :binary then []
  else              [install_cargo_zigbuild_step]
  end
end

def compile_job_name(lane)
  case lane[:lane]
  when :dmg    then "Compile #{lane[:target]} DMG"
  when :binary then "Compile #{lane[:target]} Binaries"
  else              "Compile #{lane[:target]}"
  end
end

def compile_job(lane)
  {
    "name" => compile_job_name(lane),
    "runs-on" => lane[:os],
    "needs" => "prepare_#{lane[:id]}",
    "steps" => compile_steps(lane)
  }
end

def package_steps(lane)
  [
    download_raw_artifact_step(lane),
    package_artifacts_step(lane),
    upload_final_artifact_step(lane)
  ]
end

def package_job_name(lane)
  case lane[:lane]
  when :dmg    then "Package #{lane[:target]} DMG"
  when :binary then "Package #{lane[:target]} Binaries"
  else              "Package #{lane[:target]}"
  end
end

def package_job(lane)
  {
    "name" => package_job_name(lane),
    "runs-on" => "ubuntu-latest",
    "needs" => "compile_#{lane[:lane_id]}",
    "steps" => package_steps(lane)
  }
end

def release_job
  {
    "name" => "Create Release",
    "runs-on" => "ubuntu-latest",
    "needs" => LANES.map { |lane| "package_#{lane[:lane_id]}" },
    "if" => "github.ref == 'refs/heads/master'",
    "steps" => [
      {
        "name" => "Download All Artifacts",
        "uses" => DOWNLOAD_ARTIFACT_ACTION,
        "with" => {
          "pattern" => "binaries-*",
          "path" => "artifacts",
          "merge-multiple" => true
        }
      },
      { "name" => "List Artifacts (Verify)", "run" => "ls -R artifacts" },
      {
        "name" => "Update Nightly Release",
        "uses" => RELEASE_ACTION,
        "with" => {
          "name" => "Latest Build (Nightly)",
          "tag_name" => "latest",
          "prerelease" => true,
          "files" => "artifacts/**/*"
        }
      }
    ]
  }
end

# ── Workflow ───────────────────────────────────────────────────────────────────

def workflow_data
  prepare_jobs  = TARGETS.to_h { |target| ["prepare_#{target[:id]}",  prepare_job(target)] }
  compile_jobs  = LANES.to_h   { |lane|   ["compile_#{lane[:lane_id]}", compile_job(lane)] }
  package_jobs  = LANES.to_h   { |lane|   ["package_#{lane[:lane_id]}", package_job(lane)] }

  {
    "name" => "Nightly Build & Release V2",
    "on" => { "workflow_dispatch" => {} },
    "env" => { "CARGO_TERM_COLOR" => "always" },
    "permissions" => { "contents" => "write" },
    "jobs" => prepare_jobs.merge(compile_jobs, package_jobs, "release" => release_job)
  }
end

def workflow
  yaml = YAML.dump(workflow_data, line_width: 1000).sub(/\A---\n/, "")
  "# Generated by .github/workflows/nightly_v2.rb. Do not edit by hand.\n#{yaml}"
end

# ── Entry point ────────────────────────────────────────────────────────────────

check = false
OptionParser.new do |parser|
  parser.banner = "Usage: ruby .github/workflows/nightly_v2.rb [--check]"
  parser.on("--check", "Exit nonzero if nightly_v2.yml is stale") { check = true }
end.parse!

generated = workflow

if check
  abort "#{OUTPUT} does not exist. Run: ruby .github/workflows/nightly_v2.rb" unless File.exist?(OUTPUT)
  abort "#{OUTPUT} is stale. Run: ruby .github/workflows/nightly_v2.rb" unless File.read(OUTPUT) == generated
  puts "#{OUTPUT} is up to date"
else
  File.write(OUTPUT, generated)
  puts "Wrote #{OUTPUT}"
end
