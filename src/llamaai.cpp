#include "llamaai.h"

#include <QCoreApplication>
#include <QDebug>
#include <QDir>
#include <QFileInfo>

#include <cstring>
#include <mutex>
#include <string>

#if defined(__linux__) && defined(__GLIBC__)
#include <malloc.h>
#define LAZYDESKTOP_HAVE_MALLOC_TRIM 1
#endif

#if defined(Q_OS_UNIX)
#include <dlfcn.h>
#define LAZYDESKTOP_HAVE_DLOPEN 1
#endif

// llama.cpp C API
#include "ggml-backend.h"
#include "llama.h"

// Computed lazily (not a static global) since QCoreApplication::applicationDirPath()
// requires a QCoreApplication instance to already exist.
static QStringList backendSearchDirs()
{
    return {
        QCoreApplication::applicationDirPath(),
        QDir::currentPath(),
        // Some distros (e.g. Arch's llama.cpp package) install backend .so files here instead
        "/usr/bin",
        "/usr/lib/llama.cpp",
        "/usr/lib/ggml",
    };
}

#ifdef LAZYDESKTOP_HAVE_DLOPEN
// ggml_backend_load_all() unconditionally dlopen()s every backend plugin it can find,
// including ggml-vulkan (a ~70MB shared library whose device enumeration alone eagerly
// initializes a Vulkan instance/driver context). That backend is then kept resident for
// the rest of the process, which is what left the app sitting at a few hundred extra MB
// of RAM even with GPU acceleration off and no GPU work ever performed. When GPU
// acceleration is disabled we instead replicate ggml's own "pick the best candidate"
// scoring logic (see ggml_backend_load_best() in ggml-backend-reg.cpp) but scoped to just
// the CPU backend variants, so backends we will never use never get loaded.
// Returns true if a CPU backend was found and loaded.
static bool loadBestCpuBackend()
{
    QString bestPath;
    int bestScore = 0;

    for (const QString &dirPath : backendSearchDirs()) {
        QDir dir(dirPath);
        if (dirPath.isEmpty() || !dir.exists())
            continue;

        const QStringList candidates = dir.entryList(
            {QStringLiteral("libggml-cpu-*.so")}, QDir::Files);
        for (const QString &name : candidates) {
            const QString fullPath = dir.filePath(name);
            void *handle = dlopen(fullPath.toUtf8().constData(), RTLD_NOW | RTLD_LOCAL);
            if (!handle)
                continue;
            using ScoreFn = int (*)();
            auto scoreFn = reinterpret_cast<ScoreFn>(dlsym(handle, "ggml_backend_score"));
            int score = scoreFn ? scoreFn() : 0;
            dlclose(handle);
            if (score > bestScore) {
                bestScore = score;
                bestPath = fullPath;
            }
        }
    }

    if (bestPath.isEmpty())
        return false;

    return ggml_backend_load(bestPath.toUtf8().constData()) != nullptr;
}
#endif

// The ggml backend registry is a process-wide singleton: loading it dlopen()s backend
// plugins that are never unloaded, so it must only be initialized once per process
// rather than once per generation (repeated init/free per LlamaWorker was the source of
// the growing RAM usage that persisted after the model itself was unloaded).
static void ensureBackendInitialized(bool allowGpuBackends)
{
    static std::once_flag flag;
    std::call_once(flag, [allowGpuBackends]() {
        llama_backend_init();
#ifdef LAZYDESKTOP_HAVE_DLOPEN
        // Falls through to the full ggml_backend_load_all() below if no CPU backend
        // variant was found in our search paths, so inference still works even on
        // distros/layouts we don't recognize — it just costs the extra GPU-backend RAM.
        if (!allowGpuBackends && loadBestCpuBackend())
            return;
#else
        Q_UNUSED(allowGpuBackends);
#endif
        ggml_backend_load_all();
        // Fallback: some distros install backend .so files to non-standard locations
        for (const char *dir : {"/usr/bin", "/usr/lib/llama.cpp", "/usr/lib/ggml"}) {
            if (QDir(dir).exists())
                ggml_backend_load_all_from_path(dir);
        }
    });
}

// --- LlamaWorker ---

LlamaWorker::LlamaWorker(QObject *parent)
    : QObject(parent)
{
}

LlamaWorker::~LlamaWorker()
{
    unloadModel();
}

void LlamaWorker::setModelPath(const QString &path)
{
    m_modelPath = path;
}

void LlamaWorker::setPrompt(const QString &systemPrompt, const QString &userContent)
{
    m_systemPrompt = systemPrompt;
    m_userContent = userContent;
}

void LlamaWorker::cancel()
{
    m_cancelled = true;
}

void LlamaWorker::setGpuAcceleration(bool enabled)
{
    m_gpuAccel = enabled;
}

bool LlamaWorker::loadModel()
{
    if (m_model)
        return true;

    if (m_modelPath.isEmpty()) {
        emit errorOccurred("No model path configured.");
        return false;
    }

    if (!QFileInfo::exists(m_modelPath)) {
        emit errorOccurred("Model file not found: " + m_modelPath);
        return false;
    }

    ensureBackendInitialized(m_gpuAccel);

    // Configure model parameters
    llama_model_params model_params = llama_model_default_params();
    // Use GPU acceleration if available
    model_params.n_gpu_layers = m_gpuAccel ? 99 : 0;

    m_model = llama_model_load_from_file(m_modelPath.toUtf8().constData(), model_params);
    if (!m_model) {
        emit errorOccurred("Failed to load model: " + m_modelPath);
        return false;
    }

    // Configure context parameters
    llama_context_params ctx_params = llama_context_default_params();
    ctx_params.n_ctx = 8192;  // context size
    ctx_params.n_batch = 512; // batch size for prompt processing

    m_ctx = llama_init_from_model(m_model, ctx_params);
    if (!m_ctx) {
        llama_model_free(m_model);
        m_model = nullptr;
        emit errorOccurred("Failed to create context for model.");
        return false;
    }

    // Create sampler chain
    m_smpl = llama_sampler_chain_init(llama_sampler_chain_default_params());
    llama_sampler_chain_add(m_smpl, llama_sampler_init_temp(0.1f));
    llama_sampler_chain_add(m_smpl, llama_sampler_init_dist(67));

    return true;
}

void LlamaWorker::unloadModel()
{
    if (m_smpl) {
        llama_sampler_free(m_smpl);
        m_smpl = nullptr;
    }
    if (m_ctx) {
        llama_free(m_ctx);
        m_ctx = nullptr;
    }
    if (m_model) {
        llama_model_free(m_model);
        m_model = nullptr;
    }
    // The KV cache and compute buffers for large contexts are big enough that glibc's
    // allocator keeps their freed heap arenas around instead of returning them to the OS,
    // which is what left the process sitting at several hundred extra MB of RSS after a
    // model unload. Force the freed memory back to the OS now that it's actually unused.
#ifdef LAZYDESKTOP_HAVE_MALLOC_TRIM
    malloc_trim(0);
#endif
}

std::string LlamaWorker::formatPrompt() const
{
    // Use a simple chat template format
    std::string prompt;
    prompt += "<|system|>\n";
    prompt += m_systemPrompt.toStdString();
    prompt += "\n<|user|>\n";
    prompt += m_userContent.toStdString();
    prompt += "\n<|assistant|>\n";
    return prompt;
}

void LlamaWorker::run()
{
    m_cancelled = false;

    if (!loadModel())
        return;

    const std::string prompt = formatPrompt();
    const auto *vocab = llama_model_get_vocab(m_model);

    std::vector<llama_token> tokens(prompt.size() + 16);
    int n_tokens = llama_tokenize(vocab, prompt.c_str(), prompt.size(),
                                  tokens.data(), tokens.size(), true, false);
    if (n_tokens < 0) {
        // Buffer too small, resize and retry
        tokens.resize(-n_tokens);
        n_tokens = llama_tokenize(vocab, prompt.c_str(), prompt.size(),
                                  tokens.data(), tokens.size(), true, false);
    }
    if (n_tokens < 0) {
        emit errorOccurred("Failed to tokenize prompt.");
        return;
    }
    tokens.resize(n_tokens);

    // Evaluate the prompt in batches
    int n_ctx = llama_n_ctx(m_ctx);
    int n_kv_req = n_tokens;

    // Check if we need to shift context
    if (n_kv_req > n_ctx) {
        emit errorOccurred("Prompt is too long for the model context size.");
        return;
    }

    // Process prompt in batches
    for (int i = 0; i < n_tokens; i += llama_n_batch(m_ctx)) {
        int n_eval = std::min(n_tokens - i, static_cast<int>(llama_n_batch(m_ctx)));
        if (llama_decode(m_ctx, llama_batch_get_one(&tokens[i], n_eval))) {
            emit errorOccurred("Failed to evaluate prompt.");
            return;
        }
    }

    // Generate response tokens
    QString fullResponse;
    const int max_tokens = 512;
    const llama_token eos_token = llama_vocab_eos(vocab);

    for (int i = 0; i < max_tokens && !m_cancelled; i++) {
        llama_token new_token = llama_sampler_sample(m_smpl, m_ctx, -1);

        if (new_token == eos_token)
            break;

        // Decode the token
        char buf[256];
        int len = llama_token_to_piece(vocab, new_token, buf, sizeof(buf), 0, true);
        if (len < 0) {
            emit errorOccurred("Failed to decode token.");
            return;
        }

        QString tokenText = QString::fromUtf8(buf, len);
        fullResponse += tokenText;
        emit thinking(tokenText);

        // Evaluate the new token for next iteration
        if (llama_decode(m_ctx, llama_batch_get_one(&new_token, 1))) {
            emit errorOccurred("Failed to evaluate token.");
            return;
        }
    }

    if (m_cancelled) {
        emit finished(QString());
    } else {
        QString result = fullResponse.trimmed();
        if (result.startsWith(QLatin1String("<|assistant|>")))
            result.remove(0, 13);
        emit finished(result.trimmed());
    }
}

// --- LlamaAI ---

LlamaAI::LlamaAI(QObject *parent)
    : QObject(parent)
{
}

LlamaAI::~LlamaAI()
{
    cancel();
    if (m_thread) {
        m_thread->quit();
        m_thread->wait(5000);
    }
}

void LlamaAI::setModelPath(const QString &path)
{
    m_modelPath = path;
}

QString LlamaAI::modelPath() const
{
    return m_modelPath;
}

void LlamaAI::setGpuAcceleration(bool enabled)
{
    m_gpuAccel = enabled;
}

void LlamaAI::generate(const QString &systemPrompt, const QString &userContent)
{
    if (isRunning())
        cancel();

    // Clean up old thread/worker
    if (m_thread) {
        m_thread->quit();
        m_thread->wait(5000);
        delete m_thread;
        m_thread = nullptr;
    }

    m_thread = new QThread(this);
    m_worker = new LlamaWorker(); // no parent — will be moved to thread
    m_worker->setModelPath(m_modelPath);
    m_worker->setGpuAcceleration(m_gpuAccel);
    m_worker->setPrompt(systemPrompt, userContent);

    m_worker->moveToThread(m_thread);

    connect(m_thread, &QThread::started, m_worker, &LlamaWorker::run);
    connect(m_worker, &LlamaWorker::finished, this, &LlamaAI::finished);
    connect(m_worker, &LlamaWorker::errorOccurred, this, &LlamaAI::errorOccurred);
    connect(m_worker, &LlamaWorker::thinking, this, &LlamaAI::thinking);
    connect(m_worker, &LlamaWorker::finished, m_thread, &QThread::quit);
    connect(m_worker, &LlamaWorker::errorOccurred, m_thread, &QThread::quit);
    connect(m_thread, &QThread::finished, m_worker, &QObject::deleteLater);

    m_thread->start();
}

void LlamaAI::cancel()
{
    if (m_worker)
        m_worker->cancel();
}

bool LlamaAI::isRunning() const
{
    return m_thread && m_thread->isRunning();
}