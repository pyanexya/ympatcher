package dev.pyanexy.ympresence;

import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;

final class PresenceState {
    static final String ACTION_STATUS = "dev.pyanexy.ympresence.STATUS";
    static final String EXTRA_TITLE = "title";
    static final String EXTRA_ARTIST = "artist";
    static final String EXTRA_STATUS = "status";
    static final String PREFS = "ympresence";
    static final String PREF_ENABLED = "enabled";
    static final String PREF_ACTIVITY_TYPE = "activity_type";
    static final String PREF_SHOW_ARTWORK = "show_artwork";
    static final String PREF_SHOW_PROGRESS = "show_progress";
    static final String PREF_SHOW_LINK = "show_link";
    static final String PREF_SHOW_PAUSED = "show_paused";
    static final String PREF_ARTIST_FIRST = "artist_first";

    static final int ACTIVITY_PLAYING = 0;
    static final int ACTIVITY_LISTENING = 2;
    static final int ACTIVITY_WATCHING = 3;

    private PresenceState() {}

    static boolean isEnabled(Context context) {
        return preferences(context).getBoolean(PREF_ENABLED, true);
    }

    static void setEnabled(Context context, boolean enabled) {
        preferences(context).edit().putBoolean(PREF_ENABLED, enabled).apply();
    }

    static int activityType(Context context) {
        return preferences(context).getInt(PREF_ACTIVITY_TYPE, ACTIVITY_LISTENING);
    }

    static boolean option(Context context, String key, boolean defaultValue) {
        return preferences(context).getBoolean(key, defaultValue);
    }

    static void setActivityType(Context context, int type) {
        preferences(context).edit().putInt(PREF_ACTIVITY_TYPE, type).apply();
    }

    static void setOption(Context context, String key, boolean value) {
        preferences(context).edit().putBoolean(key, value).apply();
    }

    static void publish(Context context, String title, String artist, String status) {
        Intent intent = new Intent(ACTION_STATUS)
                .setPackage(context.getPackageName())
                .putExtra(EXTRA_TITLE, title)
                .putExtra(EXTRA_ARTIST, artist)
                .putExtra(EXTRA_STATUS, status);
        context.sendBroadcast(intent);
    }

    private static SharedPreferences preferences(Context context) {
        return context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }
}
