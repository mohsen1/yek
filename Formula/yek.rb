class Yek < Formula
  desc "A tool to chunk and serialize repository content for LLM consumption"
  homepage "https://github.com/mohsen1/yek"
  version "0.1.0"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/mohsen1/yek/releases/download/v#{version}/yek-aarch64-apple-darwin.tar.gz"
      sha256 "AARCH64_HASH" # Replace with actual hash
    else
      url "https://github.com/mohsen1/yek/releases/download/v#{version}/yek-x86_64-apple-darwin.tar.gz"
      sha256 "X86_64_HASH" # Replace with actual hash
    end
  else
    url "https://github.com/mohsen1/yek/releases/download/v#{version}/yek-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "LINUX_HASH" # Replace with actual hash
  end

  def install
    bin.install "yek"
  end

  test do
    system "#{bin}/yek", "--version"
  end
end 