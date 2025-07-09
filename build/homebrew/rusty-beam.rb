class RustyBeam < Formula
  desc "HTTP server that uses CSS selectors to manipulate HTML documents via Range headers"
  homepage "https://github.com/jamesaduncan/rusty-beam"
  url "https://github.com/jamesaduncan/rusty-beam/archive/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"  # This will need to be updated with the actual release SHA
  license "Apache 2.0"
  head "https://github.com/jamesaduncan/rusty-beam.git", branch: "main"

  depends_on "rust" => :build

  def install
    # Build the main binary
    system "cargo", "build", "--release"
    bin.install "target/release/rusty-beam"
    
    # Install configuration and examples
    etc.install "config/config.html" => "rusty-beam/config.html"
    pkgshare.install "examples/localhost", "examples/files"
    
    # Build and install plugins
    system "./build/scripts/build-plugins.sh"
    lib.install Dir["plugins/lib/*.{so,dylib}"] => "rusty-beam/plugins/"
    
    # Install documentation
    doc.install "README.md", "LICENSE"
  end

  def post_install
    # Ensure directories exist
    (var/"lib/rusty-beam").mkpath
    (var/"log/rusty-beam").mkpath
  end

  service do
    run [opt_bin/"rusty-beam"]
    working_dir var/"lib/rusty-beam"
    log_path var/"log/rusty-beam/rusty-beam.log"
    error_log_path var/"log/rusty-beam/rusty-beam.error.log"
    keep_alive false
  end

  test do
    # Start the server in the background
    pid = spawn bin/"rusty-beam"
    sleep 2
    
    # Test that it responds
    system "curl", "-f", "http://127.0.0.1:3000/"
    
    # Clean up
    Process.kill("TERM", pid)
    Process.wait(pid)
  end
end