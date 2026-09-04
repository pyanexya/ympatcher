package dev.pyanexy.ympresence;

import android.app.Activity;
import android.graphics.Color;
import android.graphics.Typeface;
import android.os.Bundle;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.CompoundButton;
import android.widget.LinearLayout;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.ScrollView;
import android.widget.Space;
import android.widget.Switch;
import android.widget.TextView;

public final class DiscordPresenceActivity extends Activity {
    private int primaryText;
    private int secondaryText;
    private int surface;
    private int accent;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        EmbeddedPresence.onActivity(this);
        primaryText = themeColor(android.R.attr.textColorPrimary, Color.WHITE);
        secondaryText = themeColor(android.R.attr.textColorSecondary, 0xffa7a7a7);
        surface = themeColor(android.R.attr.colorBackground, Color.BLACK);
        accent = themeColor(android.R.attr.colorAccent, 0xffffdb00);
        setContentView(content());
    }

    private View content() {
        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.setBackgroundColor(surface);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(20), dp(12), dp(20), dp(32));
        scroll.addView(root, matchWrap());

        Space handle = new Space(this);
        LinearLayout.LayoutParams handleParams = new LinearLayout.LayoutParams(dp(40), dp(4));
        handleParams.gravity = Gravity.CENTER_HORIZONTAL;
        handleParams.bottomMargin = dp(20);
        handle.setBackgroundColor(secondaryText);
        root.addView(handle, handleParams);

        root.addView(label(getString(R.string.ymp_presence_title), 30, true, primaryText));
        TextView summary = label(getString(R.string.ymp_presence_summary), 16, false, secondaryText);
        summary.setPadding(0, dp(6), 0, dp(18));
        root.addView(summary);

        root.addView(toggle(getString(R.string.ymp_presence_enable),
                EmbeddedPresence.ENABLED, false));
        root.addView(section(getString(R.string.ymp_presence_activity_type)));
        root.addView(activityTypes());
        root.addView(section(getString(R.string.ymp_presence_display)));
        root.addView(toggle(getString(R.string.ymp_presence_artwork),
                EmbeddedPresence.SHOW_ARTWORK, true));
        root.addView(toggle(getString(R.string.ymp_presence_progress),
                EmbeddedPresence.SHOW_PROGRESS, true));
        root.addView(toggle(getString(R.string.ymp_presence_link),
                EmbeddedPresence.SHOW_LINK, true));
        root.addView(toggle(getString(R.string.ymp_presence_paused),
                EmbeddedPresence.SHOW_PAUSED, false));
        root.addView(toggle(getString(R.string.ymp_presence_artist_first),
                EmbeddedPresence.ARTIST_FIRST, false));
        return scroll;
    }

    private View activityTypes() {
        RadioGroup group = new RadioGroup(this);
        group.setOrientation(RadioGroup.VERTICAL);
        int selected = EmbeddedPresence.preferences().getInt(EmbeddedPresence.ACTIVITY_TYPE, 2);
        addActivity(group, getString(R.string.ymp_presence_listening), 2, selected);
        addActivity(group, getString(R.string.ymp_presence_playing), 0, selected);
        addActivity(group, getString(R.string.ymp_presence_watching), 3, selected);
        group.setOnCheckedChangeListener((view, checkedId) -> {
            View checked = view.findViewById(checkedId);
            if (checked != null && checked.getTag() instanceof Integer) {
                EmbeddedPresence.preferences().edit()
                        .putInt(EmbeddedPresence.ACTIVITY_TYPE, (Integer) checked.getTag())
                        .apply();
                EmbeddedPresence.refresh();
            }
        });
        return group;
    }

    private void addActivity(RadioGroup group, String title, int value, int selected) {
        RadioButton button = new RadioButton(this);
        button.setId(View.generateViewId());
        button.setTag(value);
        button.setText(title);
        button.setTextColor(primaryText);
        button.setTextSize(TypedValue.COMPLEX_UNIT_SP, 16);
        button.setGravity(Gravity.CENTER_VERTICAL);
        button.setMinHeight(dp(52));
        button.setButtonTintList(android.content.res.ColorStateList.valueOf(accent));
        group.addView(button, matchWrap());
        button.setChecked(value == selected);
    }

    private View toggle(String title, String key, boolean defaultValue) {
        Switch control = new Switch(this);
        control.setText(title);
        control.setTextColor(primaryText);
        control.setTextSize(TypedValue.COMPLEX_UNIT_SP, 16);
        control.setGravity(Gravity.CENTER_VERTICAL);
        control.setMinHeight(dp(56));
        control.setChecked(EmbeddedPresence.preferences().getBoolean(key, defaultValue));
        control.setOnCheckedChangeListener((CompoundButton button, boolean checked) -> {
            EmbeddedPresence.preferences().edit().putBoolean(key, checked).apply();
            EmbeddedPresence.refresh();
        });
        return control;
    }

    private TextView section(String value) {
        TextView label = label(value, 21, true, primaryText);
        label.setPadding(0, dp(24), 0, dp(8));
        return label;
    }

    private TextView label(String value, int size, boolean bold, int color) {
        TextView view = new TextView(this);
        view.setText(value);
        view.setTextSize(TypedValue.COMPLEX_UNIT_SP, size);
        view.setTextColor(color);
        if (bold) {
            view.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        }
        return view;
    }

    private int themeColor(int attribute, int fallback) {
        TypedValue value = new TypedValue();
        return getTheme().resolveAttribute(attribute, value, true) ? value.data : fallback;
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }

    private static ViewGroup.LayoutParams matchWrap() {
        return new ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT);
    }
}
