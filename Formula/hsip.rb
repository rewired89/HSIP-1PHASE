# Homebrew formula for HSIP
#
# To install from this tap:
#   brew tap rewired89/hsip https://github.com/rewired89/HSIP-1PHASE
#   brew install hsip
#
# Or install directly from the formula file (for local testing):
#   brew install --formula ./Formula/hsip.rb
#
# NOTE: Replace the sha256 values below with actual checksums once a release
# is published. Generate them with:
#   curl -sSfL <url> | shasum -a 256

class Hsip < Formula
  desc "Local identity server — block trackers, sign messages, control AI agents. No cloud."
  homepage "https://github.com/rewired89/HSIP-1PHASE"
  version "0.2.0"
  license "MIT"

  # Binaries are downloaded from GitHub Releases.
  # Update these URLs and sha256 hashes when a new release is published.
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/rewired89/HSIP-1PHASE/releases/download/v#{version}/hsip-macos-arm64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      url "https://github.com/rewired89/HSIP-1PHASE/releases/download/v#{version}/hsip-macos-x64"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    url "https://github.com/rewired89/HSIP-1PHASE/releases/download/v#{version}/hsip-linux-x64"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end

  def install
    # Rename the downloaded binary to 'hsip' and install it
    binary = Dir["hsip-*"].first || "hsip"
    bin.install binary => "hsip"
  end

  def post_install
    # Create config directory
    (var/"hsip").mkpath
    (etc/"hsip").mkpath
  end

  def caveats
    <<~EOS
      HSIP has been installed.

      To start HSIP:
        hsip

      Your browser will open automatically at http://127.0.0.1:7777

      Your API key is saved to:
        ~/.hsip/admin.key

      API reference (once running):
        http://127.0.0.1:7777/docs

      To run HSIP as a background service:
        brew services start hsip

      To stop:
        brew services stop hsip
    EOS
  end

  service do
    run [opt_bin/"hsip"]
    keep_alive false
    log_path var/"log/hsip.log"
    error_log_path var/"log/hsip.error.log"
    working_dir var/"hsip"
  end

  test do
    # Verify the binary runs and exits cleanly
    assert_match "hsip", shell_output("#{bin}/hsip --help 2>&1", 0)
  end
end
