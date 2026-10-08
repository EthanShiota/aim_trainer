#! /usr/bin/env nu
def main [] {}
def "main windows" [] {
  let package_dir = "windows_release"
  cargo xwin build -r --target x86_64-pc-windows-msvc
  mkdir $package_dir
  cp target/x86_64-pc-windows-msvc/release/aim-game.exe $"($package_dir)/aim_game.exe"
  rsync assets $package_dir -r
  mkdir $"($package_dir)/osu_beatmaps"
  7z a windows.zip $package_dir
}

def "main linux" [] {
  let package_dir = "linux_release"
  cargo build --release
  mkdir $package_dir
  cp target/release/aim-game $"($package_dir)/aim_game"
  rsync assets $package_dir -r
  mkdir $"($package_dir)/osu_beatmaps"
  7z a linux.zip $package_dir
}
