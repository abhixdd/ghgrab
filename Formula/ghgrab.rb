class Ghgrab < Formula
  desc "Browse and download files from Git forges without cloning"
  homepage "https://github.com/abhixdd/ghgrab"
  version "2.0.1"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/abhixdd/ghgrab/releases/download/v2.0.1/ghgrab-darwin"
      sha256 "2a9851e953e4d9181936af479c236d499531d2fc725af572ec8b6fcb7884911f"
    end
    on_arm do
      url "https://github.com/abhixdd/ghgrab/releases/download/v2.0.1/ghgrab-darwin-arm64"
      sha256 "cc8619f45f5ba315ee89ead484b0df75c905336bfeb23da767f596cb524900d0"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/abhixdd/ghgrab/releases/download/v2.0.1/ghgrab-linux"
      sha256 "fa96f467f54efb5c1b65d69e950b5c8815c6a19a8eec35e7495e34967fdf34f5"
    end
    on_arm do
      url "https://github.com/abhixdd/ghgrab/releases/download/v2.0.1/ghgrab-linux-arm64"
      sha256 "8354aa41a5822b6db1ec9fb07e8c9fd70c46548b98109b10f6700b25f1c21e10"
    end
  end

  def install
    if OS.mac?
      if Hardware::CPU.intel?
        bin.install "ghgrab-darwin" => "ghgrab"
      else
        bin.install "ghgrab-darwin-arm64" => "ghgrab"
      end
    elsif Hardware::CPU.intel?
      bin.install "ghgrab-linux" => "ghgrab"
    else
      bin.install "ghgrab-linux-arm64" => "ghgrab"
    end
  end

  test do
    output = shell_output("#{bin}/ghgrab agent tree not-a-url 2>&1", 1)
    assert_match '"ok":false', output
    assert_match '"api_version":"1"', output
  end
end
