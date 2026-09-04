package dev.pyanexy.ympresence;

public final class NativePresence {
    static {
        System.loadLibrary("ympresence");
    }

    private NativePresence() {}

    public static native boolean start(long applicationId);

    public static native void update(
            String title,
            String artist,
            String artworkUrl,
            String trackUrl,
            int activityType,
            long positionMs,
            long durationMs,
            boolean playing);

    public static native void clear();

    public static native void stop();
}
