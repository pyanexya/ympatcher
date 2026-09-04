#include <jni.h>
#include <android/log.h>

#include <algorithm>
#include <atomic>
#include <chrono>
#include <cstdint>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

#define DISCORDPP_IMPLEMENTATION
#include <discordpp.h>

namespace {
constexpr char kTag[] = "ympresence";

std::mutex g_mutex;
std::shared_ptr<discordpp::Client> g_client;
std::atomic<bool> g_running{false};

std::string from_jstring(JNIEnv* env, jstring value) {
    if (value == nullptr) {
        return {};
    }
    const char* chars = env->GetStringUTFChars(value, nullptr);
    if (chars == nullptr) {
        return {};
    }
    std::string result(chars);
    env->ReleaseStringUTFChars(value, chars);
    return result;
}

void callback_loop() {
    while (g_running.load()) {
        discordpp::RunCallbacks();
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
}

void stop_locked() {
    if (g_client) {
        g_client->ClearRichPresence();
        g_client.reset();
    }
}
}  // namespace

extern "C" JNIEXPORT jboolean JNICALL
Java_dev_pyanexy_ympresence_NativePresence_start(
        JNIEnv*, jclass, jlong application_id) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_client) {
        g_client = std::make_shared<discordpp::Client>();
        g_client->SetApplicationId(static_cast<uint64_t>(application_id));
        if (!g_running.exchange(true)) {
            std::thread(callback_loop).detach();
        }
        __android_log_print(ANDROID_LOG_INFO, kTag, "Discord RPC client started");
    }
    return JNI_TRUE;
}

extern "C" JNIEXPORT void JNICALL
Java_dev_pyanexy_ympresence_NativePresence_update(
        JNIEnv* env,
        jclass,
        jstring title,
        jstring artist,
        jstring artwork_url,
        jstring track_url,
        jint activity_type,
        jlong position_ms,
        jlong duration_ms,
        jboolean playing) {
    const std::string title_value = from_jstring(env, title);
    const std::string artist_value = from_jstring(env, artist);
    const std::string artwork_value = from_jstring(env, artwork_url);
    const std::string track_value = from_jstring(env, track_url);

    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_client || title_value.empty()) {
        return;
    }

    discordpp::Activity activity;
    const int safe_activity_type =
            activity_type == 0 || activity_type == 2 || activity_type == 3
                    ? activity_type
                    : 2;
    activity.SetType(static_cast<discordpp::ActivityTypes>(safe_activity_type));
    activity.SetName("Яндекс Музыка");
    activity.SetDetails(title_value);
    activity.SetState(artist_value);
    if (!track_value.empty()) {
        activity.SetDetailsUrl(track_value);
    }

    if (!artwork_value.empty()) {
        discordpp::ActivityAssets assets;
        assets.SetLargeImage(artwork_value);
        assets.SetLargeText(title_value + " — " + artist_value);
        activity.SetAssets(std::move(assets));
    }

    if (playing == JNI_TRUE && duration_ms > 0) {
        const auto now = std::chrono::system_clock::now();
        const auto now_seconds = std::chrono::duration_cast<std::chrono::seconds>(
                now.time_since_epoch()).count();
        const uint64_t start = static_cast<uint64_t>(
                std::max<int64_t>(0, now_seconds - position_ms / 1000));
        discordpp::ActivityTimestamps timestamps;
        timestamps.SetStart(start);
        timestamps.SetEnd(start + static_cast<uint64_t>(duration_ms / 1000));
        activity.SetTimestamps(std::move(timestamps));
    }

    g_client->UpdateRichPresence(
            std::move(activity),
            [](discordpp::ClientResult result) {
                __android_log_print(
                        result.Successful() ? ANDROID_LOG_INFO : ANDROID_LOG_ERROR,
                        kTag,
                        "Rich Presence update: %s",
                        result.Successful() ? "ok" : "failed");
            });
}

extern "C" JNIEXPORT void JNICALL
Java_dev_pyanexy_ympresence_NativePresence_clear(JNIEnv*, jclass) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (g_client) {
        g_client->ClearRichPresence();
    }
}

extern "C" JNIEXPORT void JNICALL
Java_dev_pyanexy_ympresence_NativePresence_stop(JNIEnv*, jclass) {
    {
        std::lock_guard<std::mutex> lock(g_mutex);
        stop_locked();
        g_running.store(false);
    }
    __android_log_print(ANDROID_LOG_INFO, kTag, "Discord RPC client stopped");
}
