#include "pty_process.h"
#include <jni.h>
#include <cerrno>
#include <cstring>
#include <memory>
#include <mutex>
#include <string>
#include <unordered_map>
#include <vector>

namespace {
std::mutex sessions_lock;
std::unordered_map<jlong, std::shared_ptr<workflow_pty>> sessions;
jlong next_handle = 1;

void fail(JNIEnv *env, const std::string &message) {
    jclass error = env->FindClass("java/io/IOException");
    if (error) env->ThrowNew(error, message.c_str());
}
std::shared_ptr<workflow_pty> lookup(JNIEnv *env, jlong handle) {
    std::lock_guard<std::mutex> guard(sessions_lock);
    auto found = sessions.find(handle);
    if (found == sessions.end()) { fail(env, "Terminal has already been released"); return {}; }
    return found->second;
}
std::string bytes(JNIEnv *env, jbyteArray array) {
    if (!array) { fail(env, "Null process argument"); return {}; }
    auto length = env->GetArrayLength(array);
    if (length > 1048576) { fail(env, "Process argument exceeds the size limit"); return {}; }
    std::string result(static_cast<size_t>(length), '\0');
    if (length) env->GetByteArrayRegion(array, 0, length, reinterpret_cast<jbyte *>(result.data()));
    if (result.find('\0') != std::string::npos) fail(env, "NUL is not allowed in process arguments");
    return result;
}
std::vector<std::string> strings(JNIEnv *env, jobjectArray array) {
    std::vector<std::string> result;
    if (!array) { fail(env, "Missing process arguments"); return result; }
    auto count = env->GetArrayLength(array);
    if (count > 1024) { fail(env, "Too many process arguments"); return result; }
    result.reserve(static_cast<size_t>(count));
    for (jsize i = 0; i < count && !env->ExceptionCheck(); ++i) {
        auto item = static_cast<jbyteArray>(env->GetObjectArrayElement(array, i));
        result.push_back(bytes(env, item));
        env->DeleteLocalRef(item);
    }
    return result;
}
std::vector<char *> pointers(std::vector<std::string> &values) {
    std::vector<char *> result;
    result.reserve(values.size() + 1);
    for (auto &value : values) result.push_back(value.data());
    result.push_back(nullptr);
    return result;
}
}

extern "C" JNIEXPORT jlongArray JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_spawn(JNIEnv *env, jclass,
        jbyteArray cwd_bytes, jobjectArray arguments, jobjectArray environment, jint rows, jint columns) {
    if (rows < 1 || rows > 1000 || columns < 1 || columns > 1000) { fail(env, "Invalid terminal dimensions"); return nullptr; }
    auto cwd = bytes(env, cwd_bytes);
    auto args = strings(env, arguments);
    auto vars = strings(env, environment);
    if (env->ExceptionCheck()) return nullptr;
    if (args.empty() || args.front().empty()) { fail(env, "No executable specified"); return nullptr; }
    auto argv = pointers(args);
    auto envp = pointers(vars);
    jlongArray returned = env->NewLongArray(2);
    if (!returned) return nullptr;
    workflow_pty *raw = nullptr;
    workflow_pty_error error = {};
    if (workflow_pty_spawn(cwd.c_str(), argv.data(), envp.data(), static_cast<unsigned short>(rows),
                           static_cast<unsigned short>(columns), &raw, &error) < 0) {
        fail(env, "PTY start failed at stage " + std::to_string(error.stage) + ": " + std::strerror(error.error_number));
        return nullptr;
    }
    std::shared_ptr<workflow_pty> session(raw, workflow_pty_destroy);
    jlong handle;
    {
        std::lock_guard<std::mutex> guard(sessions_lock);
        handle = next_handle++;
        sessions.emplace(handle, session);
    }
    jlong values[] = {handle, static_cast<jlong>(workflow_pty_pid(raw))};
    env->SetLongArrayRegion(returned, 0, 2, values);
    return returned;
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_read(JNIEnv *env, jclass, jlong handle, jint timeout_ms) {
    auto session = lookup(env, handle);
    if (!session) return nullptr;
    char buffer[16384];
    auto count = workflow_pty_read(session.get(), buffer, sizeof(buffer), timeout_ms < 0 ? 0 : timeout_ms > 1000 ? 1000 : timeout_ms);
    if (count == 0) return nullptr;
    if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK) { fail(env, "PTY read failed: " + std::string(std::strerror(errno))); return nullptr; }
    if (count < 0) count = 0;
    jbyteArray result = env->NewByteArray(static_cast<jsize>(count));
    if (result && count) env->SetByteArrayRegion(result, 0, static_cast<jsize>(count), reinterpret_cast<const jbyte *>(buffer));
    return result;
}

extern "C" JNIEXPORT jint JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_write(JNIEnv *env, jclass, jlong handle, jbyteArray data) {
    auto session = lookup(env, handle);
    if (!session) return -1;
    const auto length = env->GetArrayLength(data);
    if (length > 65536) { fail(env, "Terminal input chunk exceeds 64 KiB"); return -1; }
    std::vector<jbyte> buffer(static_cast<size_t>(length));
    if (length) env->GetByteArrayRegion(data, 0, length, buffer.data());
    if (env->ExceptionCheck()) return -1;
    auto count = workflow_pty_write(session.get(), buffer.data(), buffer.size(), 5000);
    if (count < 0) { fail(env, "PTY write failed: " + std::string(std::strerror(errno))); return -1; }
    return static_cast<jint>(count);
}

extern "C" JNIEXPORT void JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_resize(JNIEnv *env, jclass, jlong handle, jint rows, jint columns) {
    auto session = lookup(env, handle);
    if (!session) return;
    if (rows < 1 || rows > 1000 || columns < 1 || columns > 1000) { fail(env, "Invalid terminal dimensions"); return; }
    if (workflow_pty_resize(session.get(), static_cast<unsigned short>(rows), static_cast<unsigned short>(columns)) < 0)
        fail(env, "PTY resize failed: " + std::string(std::strerror(errno)));
}

extern "C" JNIEXPORT jint JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_waitFor(JNIEnv *env, jclass, jlong handle) {
    auto session = lookup(env, handle);
    if (!session) return 255;
    int status = 255;
    if (workflow_pty_wait(session.get(), &status) < 0) fail(env, "PTY wait failed: " + std::string(std::strerror(errno)));
    return status;
}

extern "C" JNIEXPORT void JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_stop(JNIEnv *env, jclass, jlong handle, jint grace_ms) {
    auto session = lookup(env, handle);
    if (!session) return;
    workflow_pty_stop(session.get(), grace_ms < 0 ? 0 : grace_ms > 2000 ? 2000 : grace_ms);
}

extern "C" JNIEXPORT void JNICALL
Java_top_flysoftbeta_workflow_platform_pty_NativePty_release(JNIEnv *, jclass, jlong handle) {
    std::lock_guard<std::mutex> guard(sessions_lock);
    sessions.erase(handle);
}
