#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "yaml"

OUTPUT = File.join(__dir__, "nightly_v2.yml")

TARGETS = [
  { id: "linux_x86_64", target: "x86_64-unknown-linux-musl", os: "ubuntu-latest", kind: :zig },
  { id: "linux_aarch64", target: "aarch64-unknown-linux-musl", os: "ubuntu-latest", kind: :zig },
  { id: "windows_x86_64", target: "x86_64-pc-windows-gnu", os: "ubuntu-latest", kind: :windows },
  { id: "windows_aarch64", target: "aarch64-pc-windows-gnullvm", os: "ubuntu-latest", kind: :windows },
  { id: "macos_x86_64", target: "x86_64-apple-darwin", os: "macos-latest", kind: :macos },
  { id: "macos_aarch64", target: "aarch64-apple-darwin", os: "macos-latest", kind: :macos }
].freeze

LANES = TARGETS.flat_map do |target_config|
  if target_config.fetch(:kind) == :macos
    [
      target_config.merge(lane: :dmg, lane_id: "#{target_config.fetch(:id)}_dmg"),
      target_config.merge(lane: :binary, lane_id: "#{target_config.fetch(:id)}_binary")
    ]
  else
    [target_config.merge(lane: :binaries, lane_id: target_config.fetch(:id))]
  end
end.freeze

options = { check: false }
OptionParser.new do |parser|
  parser.banner = "Usage: ruby .github/workflows/nightly_v2.rb [--check]"
  parser.on("--check", "Exit nonzero if nightly_v2.yml is stale") do
    options[:check] = true
  end
end.parse!

def command(*lines)
  lines.join("\n")
end

def checkout_step
  { "uses" => "actions/checkout@v4" }
end

def cache_step(target)
  {
    "name" => "Restore Toolchain and Cargo Cache",
    "uses" => "actions/cache@v4",
    "with" => {
      "path" => [
        "~/.cargo/bin",
        "~/.cargo/git",
        "~/.cargo/registry",
        "~/.rustup/settings.toml",
        "~/.rustup/toolchains",
        "~/.rustup/update-hashes"
      ].join("\n"),
      "key" => "${{ runner.os }}-#{target}-prepare-v1-${{ hashFiles('Cargo.lock') }}",
      "restore-keys" => "${{ runner.os }}-#{target}-prepare-v1-"
    }
  }
end

def rust_toolchain_step(target)
  {
    "name" => "Install Rust Toolchain",
    "run" => command(
      "rustup update stable",
      "rustup target add #{target}"
    )
  }
end

def cargo_fetch_step
  {
    "name" => "Fetch Cargo Dependencies",
    "run" => "cargo fetch --locked"
  }
end

def install_zig_step
  {
    "name" => "Install Zig",
    "uses" => "goto-bus-stop/setup-zig@v2",
    "with" => {
      "version" => "0.13.0"
    }
  }
end

def install_mingw_step
  {
    "name" => "Install MinGW-w64 (Required for Windows GNU targets)",
    "run" => command(
      "sudo apt-get update",
      "sudo apt-get install -y mingw-w64"
    )
  }
end

def install_cargo_zigbuild_step
  {
    "name" => "Install cargo-zigbuild",
    "run" => command(
      "if ! command -v cargo-zigbuild &> /dev/null; then",
      "  cargo install cargo-zigbuild",
      "fi"
    )
  }
end

def install_cargo_bundle_step
  {
    "name" => "Install cargo-bundle",
    "run" => command(
      "if ! command -v cargo-bundle &> /dev/null; then",
      "  cargo install cargo-bundle --version 0.11.0 --locked",
      "fi"
    )
  }
end

def prepare_tool_steps(target_config)
  if target_config.fetch(:kind) == :macos
    [install_cargo_bundle_step]
  else
    [install_cargo_zigbuild_step]
  end
end

def compile_tool_steps(lane)
  case lane.fetch(:lane)
  when :dmg
    [install_cargo_bundle_step]
  when :binary
    []
  else
    [install_cargo_zigbuild_step]
  end
end

def build_cache_step(target)
  {
    "name" => "Restore Rust Build Cache",
    "uses" => "Swatinem/rust-cache@v2",
    "with" => {
      "key" => target,
      "cache-on-failure" => true
    }
  }
end

def compile_step(target)
  {
    "name" => "Compile",
    "run" => "cargo zigbuild --target #{target} --release"
  }
end

def compile_macos_binary_step(target)
  {
    "name" => "Compile DiagramIDE Binary",
    "run" => "cargo build --release --package diagramide --target #{target}"
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

def collect_raw_artifacts_step(lane)
  target = lane.fetch(:target)

  {
    "name" => "Collect Raw Artifacts",
    "shell" => "bash",
    "run" => case lane.fetch(:lane)
             when :dmg
               command(
                 "mkdir -p dist/raw/#{lane.fetch(:lane_id)}",
                 "cp target/#{target}/release/#{target}-DiagramIDE.dmg dist/raw/#{lane.fetch(:lane_id)}/"
               )
             when :binary
               command(
                 "mkdir -p dist/raw/#{lane.fetch(:lane_id)}",
                 "cp target/#{target}/release/diagramide dist/raw/#{lane.fetch(:lane_id)}/"
               )
             else
               command(
                 "mkdir -p dist/raw/#{lane.fetch(:lane_id)}",
                 "cp target/#{target}/release/pikchr_pl* dist/raw/#{lane.fetch(:lane_id)}/",
                 "cp target/#{target}/release/pikchr_pro* dist/raw/#{lane.fetch(:lane_id)}/",
                 "cp target/#{target}/release/diagramide* dist/raw/#{lane.fetch(:lane_id)}/"
               )
             end
  }
end

def upload_raw_artifact_step(lane)
  {
    "name" => "Upload Raw Artifact",
    "uses" => "actions/upload-artifact@v4",
    "with" => {
      "name" => "raw-#{lane.fetch(:lane_id)}",
      "path" => "dist/raw/#{lane.fetch(:lane_id)}/*",
      "retention-days" => 1
    }
  }
end

def download_raw_artifact_step(lane)
  {
    "name" => "Download Raw Artifact",
    "uses" => "actions/download-artifact@v4",
    "with" => {
      "name" => "raw-#{lane.fetch(:lane_id)}",
      "path" => "dist/raw/#{lane.fetch(:lane_id)}"
    }
  }
end

def package_artifacts_step(lane)
  target = lane.fetch(:target)

  {
    "name" => "Package Artifacts",
    "shell" => "bash",
    "run" => case lane.fetch(:lane)
             when :dmg
               command(
                 "mkdir -p dist/final/#{lane.fetch(:lane_id)}",
                 "cp dist/raw/#{lane.fetch(:lane_id)}/#{target}-DiagramIDE.dmg dist/final/#{lane.fetch(:lane_id)}/"
               )
             when :binary
               command(
                 "mkdir -p dist/final/#{lane.fetch(:lane_id)}",
                 "mv dist/raw/#{lane.fetch(:lane_id)}/diagramide \\",
                 "  dist/final/#{lane.fetch(:lane_id)}/#{target}-diagramide"
               )
             else
               command(
                 "mkdir -p dist/final/#{lane.fetch(:lane_id)}",
                 "cd dist/raw/#{lane.fetch(:lane_id)}",
                 "",
                 "if [ -f \"pikchr_pl.exe\" ]; then",
                 "  mv pikchr_pl.exe  ../../final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pl.exe",
                 "  mv pikchr_pro.exe ../../final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pro.exe",
                 "  mv diagramide.exe ../../final/#{lane.fetch(:lane_id)}/#{target}-diagramide.exe",
                 "else",
                 "  mv pikchr_pl  ../../final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pl",
                 "  mv pikchr_pro ../../final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pro",
                 "  mv diagramide ../../final/#{lane.fetch(:lane_id)}/#{target}-diagramide",
                 "fi"
               )
             end
  }
end

def final_artifact_name(lane)
  case lane.fetch(:lane)
  when :dmg
    "binaries-#{lane.fetch(:target)}-dmg"
  when :binary
    "binaries-#{lane.fetch(:target)}-binary"
  else
    "binaries-#{lane.fetch(:target)}"
  end
end

def upload_final_artifact_step(lane)
  target = lane.fetch(:target)
  path = case lane.fetch(:lane)
         when :dmg
           "dist/final/#{lane.fetch(:lane_id)}/#{target}-DiagramIDE.dmg"
         when :binary
           "dist/final/#{lane.fetch(:lane_id)}/#{target}-diagramide"
         else
           [
             "dist/final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pl*",
             "dist/final/#{lane.fetch(:lane_id)}/#{target}-pikchr.pro*",
             "dist/final/#{lane.fetch(:lane_id)}/#{target}-diagramide*",
             "!**/*.d",
             "!**/*.rlib"
           ].join("\n")
         end

  {
    "name" => "Upload Final Artifact",
    "uses" => "actions/upload-artifact@v4",
    "with" => {
      "name" => final_artifact_name(lane),
      "path" => path
    }
  }
end

def prepare_steps(target_config)
  target = target_config.fetch(:target)

  [
    checkout_step,
    cache_step(target),
    rust_toolchain_step(target),
    cargo_fetch_step,
    *prepare_tool_steps(target_config)
  ]
end

def prepare_job(target_config)
  target = target_config.fetch(:target)

  {
    "name" => "Prepare #{target}",
    "runs-on" => target_config.fetch(:os),
    "steps" => prepare_steps(target_config)
  }
end

def compile_action_step(lane)
  case lane.fetch(:lane)
  when :dmg
    bundle_dmg_step(lane.fetch(:target))
  when :binary
    compile_macos_binary_step(lane.fetch(:target))
  else
    compile_step(lane.fetch(:target))
  end
end

def compile_steps(lane)
  macos = lane.fetch(:kind) == :macos

  steps = [checkout_step]
  steps << install_mingw_step if lane.fetch(:kind) == :windows
  steps << cache_step(lane.fetch(:target))
  steps << install_zig_step unless macos
  steps << rust_toolchain_step(lane.fetch(:target))
  steps.concat(compile_tool_steps(lane))
  steps << build_cache_step(lane.fetch(:target))
  steps << compile_action_step(lane)
  steps << collect_raw_artifacts_step(lane)
  steps << upload_raw_artifact_step(lane)
end

def compile_job(lane)
  {
    "name" => compile_job_name(lane),
    "runs-on" => lane.fetch(:os),
    "needs" => "prepare_#{lane.fetch(:id)}",
    "steps" => compile_steps(lane)
  }
end

def compile_job_name(lane)
  case lane.fetch(:lane)
  when :dmg
    "Compile #{lane.fetch(:target)} DMG"
  when :binary
    "Compile #{lane.fetch(:target)} Binary"
  else
    "Compile #{lane.fetch(:target)}"
  end
end

def package_steps(lane)
  [
    download_raw_artifact_step(lane),
    package_artifacts_step(lane),
    upload_final_artifact_step(lane)
  ]
end

def package_job(lane)
  {
    "name" => package_job_name(lane),
    "runs-on" => "ubuntu-latest",
    "needs" => "compile_#{lane.fetch(:lane_id)}",
    "steps" => package_steps(lane)
  }
end

def package_job_name(lane)
  case lane.fetch(:lane)
  when :dmg
    "Package #{lane.fetch(:target)} DMG"
  when :binary
    "Package #{lane.fetch(:target)} Binary"
  else
    "Package #{lane.fetch(:target)}"
  end
end

def release_job
  {
    "name" => "Create Release",
    "runs-on" => "ubuntu-latest",
    "needs" => LANES.map { |lane| "package_#{lane.fetch(:lane_id)}" },
    "if" => "github.ref == 'refs/heads/master'",
    "steps" => [
      {
        "name" => "Download All Artifacts",
        "uses" => "actions/download-artifact@v4",
        "with" => {
          "pattern" => "binaries-*",
          "path" => "artifacts",
          "merge-multiple" => true
        }
      },
      {
        "name" => "List Artifacts (Verify)",
        "run" => "ls -R artifacts"
      },
      {
        "name" => "Update Nightly Release",
        "uses" => "softprops/action-gh-release@v2",
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

def workflow_data
  prepare_jobs = TARGETS.to_h do |target_config|
    ["prepare_#{target_config.fetch(:id)}", prepare_job(target_config)]
  end
  compile_jobs = LANES.to_h do |lane|
    ["compile_#{lane.fetch(:lane_id)}", compile_job(lane)]
  end
  package_jobs = LANES.to_h do |lane|
    ["package_#{lane.fetch(:lane_id)}", package_job(lane)]
  end
  jobs = prepare_jobs.merge(compile_jobs).merge(package_jobs)
  jobs["release"] = release_job

  {
    "name" => "Nightly Build & Release V2",
    "on" => {
      "workflow_dispatch" => {}
    },
    "env" => {
      "CARGO_TERM_COLOR" => "always"
    },
    "permissions" => {
      "contents" => "write"
    },
    "jobs" => jobs
  }
end

def workflow
  yaml = YAML.dump(workflow_data, line_width: 1000).sub(/\A---\n/, "")
  "# Generated by .github/workflows/nightly_v2.rb. Do not edit by hand.\n#{yaml}"
end

generated = workflow

if options[:check]
  if !File.exist?(OUTPUT)
    warn "#{OUTPUT} does not exist. Run: ruby .github/workflows/nightly_v2.rb"
    exit 1
  end

  current = File.read(OUTPUT)
  if current != generated
    warn "#{OUTPUT} is stale. Run: ruby .github/workflows/nightly_v2.rb"
    exit 1
  end

  puts "#{OUTPUT} is up to date"
else
  File.write(OUTPUT, generated)
  puts "Wrote #{OUTPUT}"
end
