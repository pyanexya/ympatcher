package dev.pyanexy.ympresence;

import android.app.Activity;
import android.content.Context;
import android.content.SharedPreferences;
import android.media.MediaMetadata;
import android.media.session.MediaController;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.net.Uri;
import android.text.TextUtils;
import android.util.Log;

public final class EmbeddedPresence {
    static final long APPLICATION_ID = 1522963615437553694L;
    static final String PREFS = "ympatcher_presence";
    static final String ENABLED = "enabled";
    static final String ACTIVITY_TYPE = "activity_type";
    static final String SHOW_ARTWORK = "show_artwork";
    static final String SHOW_PROGRESS = "show_progress";
    static final String SHOW_LINK = "show_link";
    static final String SHOW_PAUSED = "show_paused";
    static final String ARTIST_FIRST = "artist_first";

    private static Context applicationContext;
    private static Activity engineActivity;
    private static MediaSession.Token pendingToken;
    private static MediaController controller;
    private static boolean nativeStarted;
    private static boolean nativeBroken;

    private static final MediaController.Callback CALLBACK = new MediaController.Callback() {
        @Override
        public void onMetadataChanged(MediaMetadata metadata) {
            publish(metadata, controller == null ? null : controller.getPlaybackState());
        }

        @Override
        public void onPlaybackStateChanged(PlaybackState state) {
            publish(controller == null ? null : controller.getMetadata(), state);
        }

        @Override
        public void onSessionDestroyed() {
            detach();
            safeClear();
        }
    };

    private EmbeddedPresence() {}

    public static synchronized void onActivity(Activity activity) {
        if (activity == null) {
            return;
        }
        applicationContext = activity.getApplicationContext();
        engineActivity = activity;
        if (enabled() && startNative()) {
            if (pendingToken != null) {
                attach(applicationContext, pendingToken);
            }
        } else {
            safeClear();
        }
    }

    public static synchronized void attach(Context context, MediaSession.Token token) {
        if (context == null || token == null) {
            return;
        }
        applicationContext = context.getApplicationContext();
        pendingToken = token;
        if (engineActivity == null || !enabled()) {
            return;
        }
        if (!startNative()) {
            return;
        }
        if (controller != null && token.equals(controller.getSessionToken())) {
            publish(controller.getMetadata(), controller.getPlaybackState());
            return;
        }
        detach();
        controller = new MediaController(applicationContext, token);
        controller.registerCallback(CALLBACK);
        publish(controller.getMetadata(), controller.getPlaybackState());
    }

    static synchronized void refresh() {
        if (!enabled()) {
            safeClear();
            return;
        }
        if (engineActivity != null && !startNative()) {
            return;
        }
        if (controller != null) {
            publish(controller.getMetadata(), controller.getPlaybackState());
        } else if (applicationContext != null && pendingToken != null) {
            attach(applicationContext, pendingToken);
        }
    }

    static SharedPreferences preferences() {
        if (applicationContext == null) {
            throw new IllegalStateException("ympresence is not initialized");
        }
        return applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    static boolean enabled() {
        return applicationContext != null && preferences().getBoolean(ENABLED, false);
    }

    private static void publish(MediaMetadata metadata, PlaybackState playback) {
        if (metadata == null || applicationContext == null || !enabled()) {
            safeClear();
            return;
        }
        boolean playing = playback != null && playback.getState() == PlaybackState.STATE_PLAYING;
        SharedPreferences prefs = preferences();
        if (!playing && !prefs.getBoolean(SHOW_PAUSED, false)) {
            safeClear();
            return;
        }
        if (!startNative()) {
            return;
        }

        String title = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_TITLE),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_TITLE),
                "Unknown track");
        String artist = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_ARTIST),
                metadata.getString(MediaMetadata.METADATA_KEY_ALBUM_ARTIST),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_SUBTITLE),
                "Unknown artist");
        String artwork = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_ART_URI),
                metadata.getString(MediaMetadata.METADATA_KEY_ALBUM_ART_URI),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_ICON_URI),
                "");
        String mediaId = firstNonEmpty(metadata.getString(MediaMetadata.METADATA_KEY_MEDIA_ID), "");
        String mediaUri = firstNonEmpty(metadata.getString(MediaMetadata.METADATA_KEY_MEDIA_URI), "");
        String trackUrl = trackUrl(mediaId, mediaUri);
        boolean artistFirst = prefs.getBoolean(ARTIST_FIRST, false);
        boolean progress = prefs.getBoolean(SHOW_PROGRESS, true);
        long duration = progress ? Math.max(0L, metadata.getLong(MediaMetadata.METADATA_KEY_DURATION)) : 0L;
        long position = progress && playback != null ? Math.max(0L, playback.getPosition()) : 0L;

        try {
            NativePresence.update(
                artistFirst ? artist : title,
                artistFirst ? title : artist,
                prefs.getBoolean(SHOW_ARTWORK, true) ? artwork : "",
                prefs.getBoolean(SHOW_LINK, true) ? trackUrl : "",
                prefs.getInt(ACTIVITY_TYPE, 2),
                position,
                duration,
                    playing && progress);
        } catch (LinkageError | RuntimeException error) {
            nativeBroken = true;
            Log.e("ympresence", "Discord RPC update failed", error);
        }
    }

    private static synchronized void detach() {
        if (controller != null) {
            controller.unregisterCallback(CALLBACK);
            controller = null;
        }
    }

    private static boolean startNative() {
        if (nativeBroken) {
            return false;
        }
        if (nativeStarted) {
            return true;
        }
        try {
            nativeStarted = NativePresence.start(APPLICATION_ID);
            return nativeStarted;
        } catch (Throwable error) {
            nativeBroken = true;
            Log.e("ympresence", "Discord RPC native startup failed", error);
            return false;
        }
    }

    private static void safeClear() {
        if (!nativeStarted || nativeBroken) {
            return;
        }
        try {
            NativePresence.clear();
        } catch (Throwable error) {
            nativeBroken = true;
            Log.e("ympresence", "Discord RPC clear failed", error);
        }
    }

    private static String trackUrl(String mediaId, String mediaUri) {
        if (!TextUtils.isEmpty(mediaUri)) {
            Uri uri = Uri.parse(mediaUri);
            if ("http".equals(uri.getScheme()) || "https".equals(uri.getScheme())) {
                return mediaUri;
            }
        }
        if (!TextUtils.isEmpty(mediaId) && TextUtils.isDigitsOnly(mediaId)) {
            return "https://music.yandex.ru/track/" + mediaId;
        }
        return "https://music.yandex.ru/";
    }

    private static String firstNonEmpty(String... values) {
        for (String value : values) {
            if (!TextUtils.isEmpty(value)) {
                return value;
            }
        }
        return "";
    }
}
