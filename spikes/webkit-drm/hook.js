// Injected at document start in every frame: wraps the page's EME calls, logs media events, and
// on each top page's load asks WebKit which key systems it would grant.
(() => {
  const post = (kind, data) => {
    try { webkit.messageHandlers.spike.postMessage(Object.assign(
      { kind, frame: top === self ? 'top' : location.host }, data)); } catch (e) {}
  };
  const rmksa = navigator.requestMediaKeySystemAccess &&
    navigator.requestMediaKeySystemAccess.bind(navigator);
  if (rmksa) {
    navigator.requestMediaKeySystemAccess = (keySystem, configs) => {
      post('page-asks', { keySystem });
      return rmksa(keySystem, configs).then(
        access => { post('page-granted', { keySystem, config: access.getConfiguration() }); return access; },
        error => { post('page-refused', { keySystem, error: String(error) }); throw error; });
    };
  }
  if (window.WebKitMediaKeys) {
    const Prefixed = window.WebKitMediaKeys;
    window.WebKitMediaKeys = function (keySystem) {
      post('page-prefixed-keys', { keySystem });
      return new Prefixed(keySystem);
    };
    window.WebKitMediaKeys.isTypeSupported = Prefixed.isTypeSupported.bind(Prefixed);
    window.WebKitMediaKeys.prototype = Prefixed.prototype;
  }

  const lastUpdate = new WeakMap();
  const media = e => {
    const v = e.target;
    if (!(v instanceof HTMLMediaElement)) return;
    if (e.type === 'timeupdate') {
      const last = lastUpdate.get(v) || 0;
      if (Date.now() - last < 2000) return;
      lastUpdate.set(v, Date.now());
    }
    post('media', { type: e.type, muted: v.muted, time: +v.currentTime.toFixed(2), size: `${v.videoWidth}x${v.videoHeight}`,
      src: (v.currentSrc || '').slice(0, 90), error: v.error ? `${v.error.code} ${v.error.message}` : undefined,
      initDataType: e.initDataType });
  };
  for (const type of ['loadedmetadata', 'encrypted', 'webkitneedkey', 'waitingforkey', 'playing',
      'timeupdate', 'pause', 'stalled', 'error', 'ended'])
    document.addEventListener(type, media, true);

  if (top !== self) return;
  window.addEventListener('load', async () => {
    const report = { url: location.href.slice(0, 100), ua: navigator.userAgent,
      eme: !!rmksa, prefixed: !!window.WebKitMediaKeys, mse: !!window.MediaSource,
      managedMse: !!window.ManagedMediaSource,
      hls: document.createElement('video').canPlayType('application/vnd.apple.mpegurl') };
    if (window.WebKitMediaKeys) {
      for (const ks of ['com.apple.fps.1_0', 'com.apple.fps.2_0', 'com.apple.fps.3_0'])
        report[`prefixed ${ks}`] = window.WebKitMediaKeys.isTypeSupported(ks, 'video/mp4');
    }
    const configs = [
      { initDataTypes: ['sinf', 'skd', 'cenc'],
        videoCapabilities: [{ contentType: 'video/mp4; codecs="avc1.640028"' }],
        audioCapabilities: [{ contentType: 'audio/mp4; codecs="mp4a.40.2"' }] },
      { initDataTypes: ['skd'], videoCapabilities: [{ contentType: 'application/vnd.apple.mpegurl' }] },
    ];
    for (const ks of ['com.apple.fps', 'com.apple.fps.1_0', 'com.apple.fps.2_0', 'com.widevine.alpha',
        'com.microsoft.playready', 'org.w3.clearkey']) {
      if (!rmksa) break;
      try {
        const access = await rmksa(ks, configs);
        const keys = await access.createMediaKeys();
        report[ks] = `granted, initDataTypes ${access.getConfiguration().initDataTypes}, mediaKeys ${!!keys}`;
      } catch (e) { report[ks] = `refused: ${e.name}`; }
    }
    post('probe', report);
  });
})();
