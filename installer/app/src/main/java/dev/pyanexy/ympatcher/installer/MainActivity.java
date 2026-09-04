package dev.pyanexy.ympatcher.installer;

import android.app.Activity;
import android.app.PendingIntent;
import android.content.Intent;
import android.content.IntentSender;
import android.content.pm.PackageInstaller;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.Settings;
import android.util.DisplayMetrics;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;

public final class MainActivity extends Activity {
    private static final String TARGET_PACKAGE = "ru.yandex.music";
    private TextView status;
    private ProgressBar progress;
    private Button action;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(createView());
        action.setOnClickListener(v -> startInstall());
    }

    private View createView() {
        int pad = dp(24);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.CENTER);
        root.setPadding(pad, pad, pad, pad);

        TextView title = new TextView(this);
        title.setText(R.string.title);
        title.setTextSize(24);
        title.setGravity(Gravity.CENTER);
        title.setTypeface(android.graphics.Typeface.DEFAULT_BOLD);

        status = new TextView(this);
        status.setText(R.string.ready);
        status.setTextSize(16);
        status.setGravity(Gravity.CENTER);
        status.setPadding(0, dp(14), 0, dp(14));

        progress = new ProgressBar(this);
        progress.setIndeterminate(true);
        progress.setVisibility(View.GONE);

        action = new Button(this);
        action.setText(R.string.install);

        root.addView(title, new LinearLayout.LayoutParams(-1, -2));
        root.addView(status, new LinearLayout.LayoutParams(-1, -2));
        root.addView(progress, new LinearLayout.LayoutParams(-2, -2));
        root.addView(action, new LinearLayout.LayoutParams(-1, -2));
        return root;
    }

    private void startInstall() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !getPackageManager().canRequestPackageInstalls()) {
            Intent intent = new Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:" + getPackageName()));
            startActivity(intent);
            status.setText(R.string.allow_unknown_sources);
            return;
        }

        action.setEnabled(false);
        progress.setVisibility(View.VISIBLE);
        status.setText(R.string.installing);

        new Thread(() -> {
            try {
                installSelectedSplits();
                runOnUiThread(() -> status.setText(R.string.install_prompt));
            } catch (Exception error) {
                runOnUiThread(() -> {
                    progress.setVisibility(View.GONE);
                    action.setEnabled(true);
                    status.setText(getString(R.string.install_failed, error.getMessage()));
                });
            }
        }, "ympatcher-install").start();
    }

    private void installSelectedSplits() throws IOException {
        PackageInstaller installer = getPackageManager().getPackageInstaller();
        PackageInstaller.SessionParams params =
                new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
        params.setAppPackageName(TARGET_PACKAGE);

        int sessionId = installer.createSession(params);
        PackageInstaller.Session session = installer.openSession(sessionId);
        try {
            List<String> copied = copySelectedApks(session);
            if (copied.isEmpty()) {
                throw new IOException("APKS payload has no compatible APK splits");
            }
            Intent intent = new Intent(this, InstallResultReceiver.class);
            PendingIntent pending = PendingIntent.getBroadcast(
                    this,
                    sessionId,
                    intent,
                    PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_MUTABLE);
            IntentSender sender = pending.getIntentSender();
            session.commit(sender);
        } catch (Exception error) {
            session.abandon();
            throw error;
        } finally {
            session.close();
        }
    }

    private List<String> copySelectedApks(PackageInstaller.Session session) throws IOException {
        List<String> copied = new ArrayList<>();
        Set<String> selected = selectSplits();
        try (ZipInputStream zip = new ZipInputStream(getAssets().open("payload.apks"))) {
            ZipEntry entry;
            while ((entry = zip.getNextEntry()) != null) {
                String name = entry.getName();
                if (entry.isDirectory() || !name.endsWith(".apk") || name.contains("/") || name.contains("\\")) {
                    continue;
                }
                if (!isSelected(name, selected)) {
                    continue;
                }
                try (OutputStream out = session.openWrite(name, 0, entry.getSize())) {
                    copy(zip, out);
                    session.fsync(out);
                }
                copied.add(name);
            }
        }
        return copied;
    }

    private Set<String> selectSplits() {
        Set<String> selected = new HashSet<>();
        selected.add("base.apk");
        selected.add("split_config." + normalizeAbi(Build.SUPPORTED_ABIS[0]) + ".apk");
        selected.add("split_config." + bestDensity() + ".apk");
        selected.add("split_config.en.apk");
        selected.add("split_config.ru.apk");
        selected.add("split_config." + Locale.getDefault().getLanguage() + ".apk");
        return selected;
    }

    private boolean isSelected(String name, Set<String> selected) {
        if (selected.contains(name)) {
            return true;
        }
        return name.endsWith(".apk") && !name.startsWith("split_config.");
    }

    private String normalizeAbi(String abi) {
        return abi.replace("-", "_");
    }

    private String bestDensity() {
        DisplayMetrics metrics = getResources().getDisplayMetrics();
        int dpi = metrics.densityDpi;
        if (dpi <= 160) {
            return "mdpi";
        } else if (dpi <= 240) {
            return "hdpi";
        } else if (dpi <= 320) {
            return "xhdpi";
        } else if (dpi <= 480) {
            return "xxhdpi";
        }
        return "xxxhdpi";
    }

    private void copy(InputStream input, OutputStream output) throws IOException {
        byte[] buffer = new byte[1024 * 128];
        int read;
        while ((read = input.read(buffer)) != -1) {
            output.write(buffer, 0, read);
        }
    }

    private int dp(int value) {
        return (int) (value * getResources().getDisplayMetrics().density + 0.5f);
    }
}
