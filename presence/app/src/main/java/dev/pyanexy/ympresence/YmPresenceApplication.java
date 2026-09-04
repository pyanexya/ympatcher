package dev.pyanexy.ympresence;

import android.app.Application;

import com.google.android.material.color.DynamicColors;

public final class YmPresenceApplication extends Application {
    @Override
    public void onCreate() {
        super.onCreate();
        DynamicColors.applyToActivitiesIfAvailable(this);
    }
}
