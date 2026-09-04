package dev.pyanexy.ympresence;

final class NativePresence {
    static {
        System.loadLibrary("ympresence");
    }

    private NativePresence() {}

    static native boolean start(long applicationId);

    static native void update(
            String title,
            String artist,
            String artworkUrl,
            String trackUrl,
            int activityType,
            long positionMs,
            long durationMs,
            boolean playing);

    static native void clear();
}
