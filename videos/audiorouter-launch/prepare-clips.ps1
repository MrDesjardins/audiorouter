$ErrorActionPreference='Stop'
$clips=@(
 @('gate','458:476:778:275'), @('limiter','458:476:778:275'),
 @('spectral','458:352:778:438'), @('denoise','458:328:778:275'),
 @('duck','458:342:778:270'), @('bass','458:334:778:263'),
 @('receive','458:254:778:535'), @('send','458:302:778:425'),
 @('api','458:260:778:322'), @('recorder','458:430:778:310'),
 @('route-drag','690:300:32:487'), @('arrange','690:300:32:487')
)
foreach($clip in $clips){
 & ffmpeg -y -hide_banner -loglevel error -framerate 16 -i "capture/live-$($clip[0])/%04d.jpg" -vf "crop=$($clip[1])" -c:v libx264 -pix_fmt yuv420p "assets/clips/$($clip[0])-live.mp4"
 if($LASTEXITCODE -ne 0){throw "Clip encode failed: $($clip[0])"}
}
& ffmpeg -y -hide_banner -loglevel error -framerate 16 -i capture/live-mcp/%04d.jpg -vf 'crop=458:130:778:322,drawbox=x=222:y=107:w=236:h=23:color=0x27323b:t=fill' -c:v libx264 -pix_fmt yuv420p assets/clips/mcp-live.mp4
& ffmpeg -y -hide_banner -loglevel error -ss 11 -i assets/clips/reaeq-editor.mp4 -t 3 -an -c:v libx264 -pix_fmt yuv420p assets/clips/plugin-live.mp4
New-Item -ItemType Directory -Force assets/vendor | Out-Null
Copy-Item -LiteralPath node_modules/gsap/dist/gsap.min.js -Destination assets/vendor/gsap.min.js
