# Rakefile

# ── Build / Test ───────────────────────────────────────────────────────────────

desc "Run tests with output"
task :test do
  sh "cargo test -- --nocapture"
end

desc "Run tests quietly (warnings suppressed)"
task :test_quiet do
  sh 'RUSTFLAGS="-A warnings" cargo test'
end

desc "Watch .rs/.pl files and re-run tests"
task :test_watch do
  sh "fd -e rs -e pl | entr -r -c rake test"
end

desc "Watch .rs/.pl files and re-run tests quietly"
task :test_quiet_watch do
  sh "fd -e rs -e pl | entr -r -c rake test_quiet"
end

desc "Watch .rs/.pl files and rebuild"
task :build_watch do
  sh "fd -e rs -e pl | entr -r -c cargo build"
end

# ── macOS bundling ─────────────────────────────────────────────────────────────

desc "Build release .app bundle, then install to /Applications"
task :bundle do
  sh "cargo bundle -r"
  Rake::Task[:bundle_install].invoke
end

desc "Copy icon files from ~/Downloads"
task :copy_icons do
  sh "cp -f ~/Downloads/icon.png ./icon.png"
  sh "cp -f ~/Downloads/icon.svg ./icon.svg"
end

desc "Install bundled DiagramIDE.app to /Applications"
task :bundle_install do
  rm_rf "/Applications/DiagramIDE.app"
  sh "cp -pr target/release/bundle/osx/DiagramIDE.app /Applications"
end

# ── Profiling ──────────────────────────────────────────────────────────────────

desc "Launch Tracy profiler"
task :tracy do
  sh({ "TRACY_DPI_SCALE" => "1.0" }, "tracy")
end
