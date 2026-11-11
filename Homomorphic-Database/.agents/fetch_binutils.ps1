$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$pkgs = @(
  'mingw-w64-x86_64-gettext-0.22.4-3-any.pkg.tar.zst',
  'mingw-w64-x86_64-winpthreads-12.0.0.r747.g1a99f8514-1-any.pkg.tar.zst',
  'mingw-w64-x86_64-zlib-1.3.1-1-any.pkg.tar.zst'
)
$base = 'https://repo.msys2.org/mingw/mingw64/'
foreach ($p in $pkgs) {
    $out = 'C:\tmp\deps\' + $p
    New-Item -ItemType Directory -Force -Path 'C:\tmp\deps' | Out-Null
    Invoke-WebRequest ($base + $p) -OutFile $out -UseBasicParsing
    Write-Output ("DOWNLOADED " + $p + " " + (Get-Item $out).Length + " bytes")
}
