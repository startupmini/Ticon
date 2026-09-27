# Render pratinjau ke PNG memakai System.Drawing + font subset.
# Hanya untuk dilihat; artefak yang masuk repo adalah preview.svg.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$akar = Split-Path -Parent $PSScriptRoot
$ttf = Join-Path $akar '.cache-preview\subset.ttf'
$svg = Join-Path $akar 'preview.svg'
$keluar = Join-Path $env:TEMP 'ikon-pratinjau.png'

# Ambil baris yang sama dengan yang dipakai gen-preview.py: baca <text> dari SVG
$isi = [IO.File]::ReadAllText($svg, [Text.Encoding]::UTF8)
$baris = [regex]::Matches($isi, '<text x="\d+" y="\d+">(.*?)</text>') | ForEach-Object {
    $_.Groups[1].Value -replace '&amp;', '&' -replace '&lt;', '<' -replace '&gt;', '>'
}
if ($baris.Count -eq 0) { throw 'tidak ada baris teks di preview.svg' }

$koleksi = New-Object System.Drawing.Text.PrivateFontCollection
$koleksi.AddFontFile($ttf)
$famili = $koleksi.Families[0]

$lebar = 700
$tinggiBaris = 30
$tinggi = 26 + $tinggiBaris * $baris.Count

$bitmap = New-Object System.Drawing.Bitmap($lebar, $tinggi)
$g = [System.Drawing.Graphics]::FromImage($bitmap)
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$g.Clear([System.Drawing.Color]::FromArgb(13, 17, 23))
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit

$font = New-Object System.Drawing.Font($famili, 15, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$warna = [System.Drawing.Brushes]::Gainsboro

for ($i = 0; $i -lt $baris.Count; $i++) {
    $y = 26 + $i * $tinggiBaris
    $g.DrawString($baris[$i], $font, $warna, 20, $y)
}

$bitmap.Save($keluar, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bitmap.Dispose(); $font.Dispose(); $koleksi.Dispose()
"PNG: $keluar ($([math]::Round((Get-Item $keluar).Length/1KB,1)) KiB, $($baris.Count) baris)"
