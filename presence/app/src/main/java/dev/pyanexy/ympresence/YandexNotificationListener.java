package dev.pyanexy.ympresence;

import android.app.Notification;
import android.media.MediaMetadata;
import android.media.session.MediaController;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.net.Uri;
import android.os.Bundle;
import android.os.Build;
import android.service.notification.NotificationListenerService;
import android.service.notification.StatusBarNotification;
import android.text.TextUtils;

public final class YandexNotificationListener extends NotificationListenerService {
    private static final String YANDEX_PACKAGE = "ru.yandex.music";

    private MediaController controller;

    private final MediaController.Callback callback = new MediaController.Callback() {
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
            detachController();
            NativePresence.clear();
            PresenceState.publish(
                    YandexNotificationListener.this,
                    getString(R.string.no_track),
                    "",
                    getString(R.string.status_waiting_track));
        }
    };

    @Override
    public void onListenerConnected() {
        super.onListenerConnected();
        scanActiveNotifications();
    }

    @Override
    public void onNotificationPosted(StatusBarNotification sbn) {
        if (YANDEX_PACKAGE.equals(sbn.getPackageName())) {
            attachFromNotification(sbn.getNotification());
        }
    }

    @Override
    public void onNotificationRemoved(StatusBarNotification sbn) {
        if (!YANDEX_PACKAGE.equals(sbn.getPackageName())) {
            return;
        }
        NativePresence.clear();
        PresenceState.publish(
                this,
                getString(R.string.no_track),
                "",
                getString(R.string.status_waiting_track));
    }

    @Override
    public void onDestroy() {
        detachController();
        super.onDestroy();
    }

    private void scanActiveNotifications() {
        StatusBarNotification[] notifications = getActiveNotifications();
        if (notifications == null) {
            return;
        }
        for (StatusBarNotification sbn : notifications) {
            if (YANDEX_PACKAGE.equals(sbn.getPackageName())) {
                attachFromNotification(sbn.getNotification());
                return;
            }
        }
    }

    private void attachFromNotification(Notification notification) {
        Bundle extras = notification.extras;
        MediaSession.Token token;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            token = extras.getParcelable(Notification.EXTRA_MEDIA_SESSION, MediaSession.Token.class);
        } else {
            token = extras.getParcelable(Notification.EXTRA_MEDIA_SESSION);
        }
        if (token == null) {
            return;
        }
        if (controller != null && controller.getSessionToken().equals(token)) {
            publish(controller.getMetadata(), controller.getPlaybackState());
            return;
        }
        detachController();
        controller = new MediaController(this, token);
        controller.registerCallback(callback);
        publish(controller.getMetadata(), controller.getPlaybackState());
    }

    private void detachController() {
        if (controller != null) {
            controller.unregisterCallback(callback);
            controller = null;
        }
    }

    private void publish(MediaMetadata metadata, PlaybackState playback) {
        if (metadata == null || !PresenceState.isEnabled(this)) {
            NativePresence.clear();
            return;
        }

        String title = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_TITLE),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_TITLE),
                getString(R.string.unknown_track));
        String artist = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_ARTIST),
                metadata.getString(MediaMetadata.METADATA_KEY_ALBUM_ARTIST),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_SUBTITLE),
                getString(R.string.unknown_artist));
        String art = firstNonEmpty(
                metadata.getString(MediaMetadata.METADATA_KEY_ART_URI),
                metadata.getString(MediaMetadata.METADATA_KEY_ALBUM_ART_URI),
                metadata.getString(MediaMetadata.METADATA_KEY_DISPLAY_ICON_URI),
                "");
        String mediaId = firstNonEmpty(metadata.getString(MediaMetadata.METADATA_KEY_MEDIA_ID), "");
        String mediaUri = firstNonEmpty(metadata.getString(MediaMetadata.METADATA_KEY_MEDIA_URI), "");
        String trackUrl = trackUrl(mediaId, mediaUri);
        long duration = Math.max(0L, metadata.getLong(MediaMetadata.METADATA_KEY_DURATION));
        long position = playback == null ? 0L : Math.max(0L, playback.getPosition());
        boolean playing = playback != null && playback.getState() == PlaybackState.STATE_PLAYING;

        if (!playing && !PresenceState.option(this, PresenceState.PREF_SHOW_PAUSED, false)) {
            NativePresence.clear();
            PresenceState.publish(this, title, artist, getString(R.string.status_paused_hidden));
            return;
        }

        boolean artistFirst = PresenceState.option(this, PresenceState.PREF_ARTIST_FIRST, false);
        String details = artistFirst ? artist : title;
        String state = artistFirst ? title : artist;
        String visibleArt = PresenceState.option(this, PresenceState.PREF_SHOW_ARTWORK, true) ? art : "";
        String visibleLink = PresenceState.option(this, PresenceState.PREF_SHOW_LINK, true) ? trackUrl : "";
        boolean showProgress = PresenceState.option(this, PresenceState.PREF_SHOW_PROGRESS, true);

        NativePresence.update(
                details,
                state,
                visibleArt,
                visibleLink,
                PresenceState.activityType(this),
                showProgress ? position : 0L,
                showProgress ? duration : 0L,
                playing && showProgress);
        PresenceState.publish(
                this,
                title,
                artist,
                getString(playing ? R.string.status_published : R.string.status_paused));
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
