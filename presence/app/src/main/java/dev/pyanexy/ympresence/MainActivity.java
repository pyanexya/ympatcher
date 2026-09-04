package dev.pyanexy.ympresence;

import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.os.Bundle;
import android.provider.Settings;
import android.text.TextUtils;
import android.widget.TextView;

import androidx.annotation.Nullable;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.content.ContextCompat;

import com.discord.socialsdk.DiscordSocialSdkInit;
import com.discord.socialsdk.NativeCalls;
import com.google.android.material.button.MaterialButton;
import com.google.android.material.button.MaterialButtonToggleGroup;
import com.google.android.material.checkbox.MaterialCheckBox;
import com.google.android.material.materialswitch.MaterialSwitch;

public final class MainActivity extends AppCompatActivity {
    private TextView permissionState;
    private TextView discordState;
    private TextView trackTitle;
    private TextView trackArtist;
    private TextView rpcState;
    private MaterialButton permissionButton;
    private MaterialSwitch enabledSwitch;
    private MaterialButtonToggleGroup activityTypeGroup;
    private MaterialCheckBox showArtwork;
    private MaterialCheckBox showProgress;
    private MaterialCheckBox showLink;
    private MaterialCheckBox showPaused;
    private MaterialCheckBox artistFirst;

    private final BroadcastReceiver statusReceiver = new BroadcastReceiver() {
        @Override
        public void onReceive(Context context, Intent intent) {
            if (!PresenceState.ACTION_STATUS.equals(intent.getAction())) {
                return;
            }
            trackTitle.setText(intent.getStringExtra(PresenceState.EXTRA_TITLE));
            trackArtist.setText(intent.getStringExtra(PresenceState.EXTRA_ARTIST));
            rpcState.setText(intent.getStringExtra(PresenceState.EXTRA_STATUS));
        }
    };

    @Override
    protected void onCreate(@Nullable Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        DiscordSocialSdkInit.setEngineActivity(this);
        setContentView(R.layout.activity_main);

        permissionState = findViewById(R.id.permission_state);
        discordState = findViewById(R.id.discord_state);
        trackTitle = findViewById(R.id.track_title);
        trackArtist = findViewById(R.id.track_artist);
        rpcState = findViewById(R.id.rpc_state);
        permissionButton = findViewById(R.id.permission_button);
        enabledSwitch = findViewById(R.id.presence_switch);
        activityTypeGroup = findViewById(R.id.activity_type_group);
        showArtwork = findViewById(R.id.show_artwork);
        showProgress = findViewById(R.id.show_progress);
        showLink = findViewById(R.id.show_link);
        showPaused = findViewById(R.id.show_paused);
        artistFirst = findViewById(R.id.artist_first);

        enabledSwitch.setChecked(PresenceState.isEnabled(this));
        enabledSwitch.setOnCheckedChangeListener((button, checked) -> {
            PresenceState.setEnabled(this, checked);
            if (!checked) {
                NativePresence.clear();
                rpcState.setText(R.string.status_disabled);
            } else {
                startRpc();
                requestListenerRebind();
            }
        });
        permissionButton.setOnClickListener(view ->
                startActivity(new Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)));

        bindPresenceOptions();

        startRpc();
        refreshSystemState();
    }

    @Override
    protected void onStart() {
        super.onStart();
        ContextCompat.registerReceiver(
                this,
                statusReceiver,
                new IntentFilter(PresenceState.ACTION_STATUS),
                ContextCompat.RECEIVER_NOT_EXPORTED);
    }

    @Override
    protected void onStop() {
        unregisterReceiver(statusReceiver);
        super.onStop();
    }

    @Override
    protected void onResume() {
        super.onResume();
        DiscordSocialSdkInit.setEngineActivity(this);
        startRpc();
        refreshSystemState();
        if (hasNotificationAccess()) {
            requestListenerRebind();
        }
    }

    private void startRpc() {
        if (!PresenceState.isEnabled(this)) {
            rpcState.setText(R.string.status_disabled);
            return;
        }
        boolean started = NativePresence.start(BuildConfig.DISCORD_APPLICATION_ID);
        rpcState.setText(started ? R.string.status_waiting_track : R.string.status_rpc_error);
    }

    private void refreshSystemState() {
        boolean hasAccess = hasNotificationAccess();
        permissionState.setText(hasAccess ? R.string.status_access_granted : R.string.status_access_missing);
        permissionButton.setText(hasAccess ? R.string.open_access_settings : R.string.grant_access);
        discordState.setText(NativeCalls.isDiscordAppInstalled()
                ? R.string.status_discord_ready
                : R.string.status_discord_missing);
    }

    private boolean hasNotificationAccess() {
        String enabled = Settings.Secure.getString(
                getContentResolver(),
                "enabled_notification_listeners");
        if (TextUtils.isEmpty(enabled)) {
            return false;
        }
        ComponentName expected = new ComponentName(this, YandexNotificationListener.class);
        for (String value : enabled.split(":")) {
            ComponentName current = ComponentName.unflattenFromString(value);
            if (expected.equals(current)) {
                return true;
            }
        }
        return false;
    }

    private void requestListenerRebind() {
        YandexNotificationListener.requestRebind(
                new ComponentName(this, YandexNotificationListener.class));
    }

    private void bindPresenceOptions() {
        int selectedType = PresenceState.activityType(this);
        activityTypeGroup.check(selectedType == PresenceState.ACTIVITY_PLAYING
                ? R.id.activity_playing
                : selectedType == PresenceState.ACTIVITY_WATCHING
                        ? R.id.activity_watching
                        : R.id.activity_listening);
        activityTypeGroup.addOnButtonCheckedListener((group, checkedId, isChecked) -> {
            if (!isChecked) {
                return;
            }
            int type = checkedId == R.id.activity_playing
                    ? PresenceState.ACTIVITY_PLAYING
                    : checkedId == R.id.activity_watching
                            ? PresenceState.ACTIVITY_WATCHING
                            : PresenceState.ACTIVITY_LISTENING;
            PresenceState.setActivityType(this, type);
            refreshPresence();
        });

        bindOption(showArtwork, PresenceState.PREF_SHOW_ARTWORK, true);
        bindOption(showProgress, PresenceState.PREF_SHOW_PROGRESS, true);
        bindOption(showLink, PresenceState.PREF_SHOW_LINK, true);
        bindOption(showPaused, PresenceState.PREF_SHOW_PAUSED, false);
        bindOption(artistFirst, PresenceState.PREF_ARTIST_FIRST, false);
    }

    private void bindOption(MaterialCheckBox checkbox, String key, boolean defaultValue) {
        checkbox.setChecked(PresenceState.option(this, key, defaultValue));
        checkbox.setOnCheckedChangeListener((button, checked) -> {
            PresenceState.setOption(this, key, checked);
            refreshPresence();
        });
    }

    private void refreshPresence() {
        if (PresenceState.isEnabled(this) && hasNotificationAccess()) {
            requestListenerRebind();
        }
    }
}
