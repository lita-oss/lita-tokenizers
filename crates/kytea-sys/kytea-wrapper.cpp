#include <kytea/config.h>
#include <kytea/corpus-io.h>
#include <kytea/kytea.h>
#include <kytea/model-io.h>

#include <iostream>
#include <span>
// We do not include `spanstream` because it is not available in some environments yet
// #include <spanstream>
#include <sstream>
#include <string_view>
#include <fstream>

#include <cerrno>
#include <exception>
#include <cstring>

using std::iostream, std::fstream, std::stringstream;
using std::exception;

using kytea::Kytea, kytea::KyteaConfig, kytea::KyteaSentence;

using kytea::CorpusIO, kytea::CorpusFormat, kytea::ModelIO;

class ispanstream_buf_ : public std::streambuf {
public:
    explicit
    ispanstream_buf_(std::span<const char> s) {
        char* base = const_cast<char*>(s.data());
        setg(base, base, base + s.size());
    }
};

class ispanstream : public std::iostream {
public:
    explicit
    ispanstream(std::span<const char> s)
    : std::iostream(&buf_), buf_(s)
    {}
private:
    ispanstream_buf_ buf_;
};

extern "C" {
    const char* kytea_version() {
        return VERSION;
    }

    typedef struct {
        char* msg;
        int code;
    } Err;

    Err kytea_err_message(const exception& e) {
        auto msg = strdup(e.what()); 
        auto code = errno != 0 ? errno : 255;
        Err ret { .msg = msg, .code = code };
        return ret;
    }

    Err kytea_null_err() {
        errno = 0;

        Err ret {};
        return ret;
    }

    void kytea_free_err_message(char* e) {
        free(e);
    }

    Kytea* kytea_model_new() {
        return new Kytea;
    }

    void kytea_model_delete(Kytea* kytea) {
        delete kytea;
    }

    Err kytea_model_read_path(Kytea* kytea, const char* model, ModelIO::Format format) {
        auto err = kytea_null_err();
        try {
            kytea->readModel(model, format);
            return err;
        }
        catch (const exception& e) {
            err = kytea_err_message(e);
            return err;
        }
    }

    Err kytea_model_read_stream(Kytea* kytea, iostream& stream, ModelIO::Format format) {
        auto err = kytea_null_err();
        try {
            kytea->readModel(stream, format);
            return err;
        }
        catch (const exception& e) {
            err = kytea_err_message(e);
            return err;
        }
    }

    int kytea_model_sanity_train(Kytea* kytea) {
        using kytea::CORP_FORMAT_RAW;

        auto config = kytea->getConfig();

        if (!config->getDoWS() && !config->getDoTags()) {
            return 1;
        }

        if (!config->getDoWS() && config->getInputFormat() == CORP_FORMAT_RAW) {
            return 2;
        }

        if (config->getDoWS() && kytea->getWSModel() == NULL) {
            return 3;
        }

        return 0;
    }

    KyteaConfig* kytea_model_config(Kytea* kytea) {
        return kytea->getConfig();
    }

    void kytea_config_set_debug(KyteaConfig* config, unsigned level) {
        config->setDebug(level);
    }

    void kytea_config_set_training(KyteaConfig* config, bool flag) {
        config->setOnTraining(flag);
    }

    void kytea_config_set_word_bound(KyteaConfig* config, const char* word_bound) {
        config->setWordBound(word_bound);
    }
    void kytea_config_set_tag_bound(KyteaConfig* config, const char* tag_bound) {
        config->setTagBound(tag_bound);
    }
    void kytea_config_set_elem_bound(KyteaConfig* config, const char* elem_bound) {
        config->setElemBound(elem_bound);
    }
    void kytea_config_set_unk_bound(KyteaConfig* config, const char* unk_bound) {
        config->setUnkBound(unk_bound);
    }
    void kytea_config_set_no_bound(KyteaConfig* config, const char* no_bound) {
        config->setNoBound(no_bound);
    }
    void kytea_config_set_has_bound(KyteaConfig* config, const char* has_bound) {
        config->setHasBound(has_bound);
    }
    void kytea_config_set_skip_bound(KyteaConfig* config, const char* skip_bound) {
        config->setSkipBound(skip_bound);
    }
    void kytea_config_set_escape(KyteaConfig* config, const char* escape) {
        config->setEscape(escape);
    }

    void kytea_config_set_input_format(KyteaConfig* config, CorpusFormat fmt) {
        config->setInputFormat(fmt);
    }

    void kytea_config_set_do_ws(KyteaConfig* config, bool do_ws) {
        config->setDoWS(do_ws);
    }

    stringstream* kytea_stringstream_new() {
        return new stringstream;
    }

    void kytea_stringstream_delete(stringstream* buf) {
        delete buf;
    }

    typedef struct {
        const char* ptr;
        size_t size;
    } Str;

    Str kytea_stringstream_as_slice(stringstream* buf) {
        auto sv = buf->view();
        Str str { .ptr = sv.data(), .size = sv.size() };
        return str;
    }

    void kytea_stringstream_write(stringstream* buf, Str input) {
        buf->write(input.ptr, input.size);
    }

    typedef struct {
        fstream* file;
        Err err;
    } FileResult;

    FileResult kytea_fstream_new_path_in(const char* path) {
        auto mode = fstream::in;

        FileResult ret { .file = new fstream(), .err = kytea_null_err() };
        ret.file->exceptions(fstream::badbit);

        try {
            ret.file->open(path, mode);
            if (!ret.file->is_open()) {
                ret.err.code = errno != 0 ? errno : EIO;
            }
            return ret;
        }
        catch (const exception& e) {
            ret.err = kytea_err_message(e);
            return ret;
        }
    }

    FileResult kytea_fstream_new_path_out(const char* path, bool append) {
        auto mode = append ? fstream::app : fstream::out;

        FileResult ret { .file = new fstream(), .err = kytea_null_err() };
        ret.file->exceptions(fstream::badbit);

        try {
            ret.file->open(path, mode);
            if (!ret.file->is_open()) {
                ret.err.code = errno != 0 ? errno : EIO;
            }
            return ret;
        }
        catch (const exception& e) {
            ret.err = kytea_err_message(e);
            return ret;
        }
    }

    void kytea_fstream_delete(fstream* file) {
        delete file;
    }

    FileResult kytea_fstream_flush(fstream *file) {
        FileResult ret { .file = new fstream(), .err = kytea_null_err() };
        try {
            file->flush();
            return ret;
        }
        catch (const exception& e) {
            ret.err = kytea_err_message(e);
            return ret;
        }
    }

    ispanstream* kytea_ispanstream_new(const char* ptr, size_t len) {
        return new ispanstream(std::span<const char>(ptr, len));
    }

    void kytea_ispanstream_delete(ispanstream* stream) {
        delete stream;
    }

    CorpusIO* kytea_model_corpus(Kytea* kytea, iostream* corpus, bool is_output) {
        auto util = kytea->getStringUtil(); 
        auto config = kytea->getConfig();
        auto fmt = is_output ? config->getOutputFormat() : config->getInputFormat();

        auto io = CorpusIO::createIO(*corpus, fmt, *config, is_output, util);
        return io;
    }

    void kytea_corpus_io_delete(CorpusIO* corpus) {
        delete corpus;
    }

    void kytea_model_prepare_train(Kytea* kytea, CorpusIO* out) {
        using kytea::CORP_FORMAT_DEFAULT;
        using kytea::CORP_FORMAT_RAW;
        using kytea::CORP_FORMAT_TOK;

        auto config = kytea->getConfig();

        if (config->getDoWS()) {
            if(config->getInputFormat() == CORP_FORMAT_DEFAULT) {
                config->setInputFormat(CORP_FORMAT_RAW);
            }
        } else {
            if (config->getInputFormat() == CORP_FORMAT_DEFAULT) {
                config->setInputFormat(CORP_FORMAT_TOK);
            }
        }

        out->setUnkTag(config->getUnkTag());
        out->setNumTags(config->getNumTags());

        for (int i = 0; i < config->getNumTags(); i++) {
            out->setDoTag(i, config->getDoTag(i));
        }
    }

    typedef struct {
        bool ended;
        Err err;
    } PredResult;

    PredResult kytea_model_predict(Kytea* kytea, CorpusIO* in, CorpusIO* out) {
        auto config = kytea->getConfig();
        KyteaSentence* next {};
        PredResult ret { .ended = true, .err = kytea_null_err() };

        try {
            next = in->readSentence();
            if (next != NULL) {
                if (config->getDoWS()) {
                    kytea->calculateWS(*next);
                }
                if (config->getDoTags()) {
                    for (int i = 0; i < config->getNumTags(); i++) {
                        if (config->getDoTag(i)) {
                            kytea->calculateTags(*next, i);
                        }
                    }
                }
                out->writeSentence(next);
                delete next;

                ret.ended = false;
            } else {
                ret.ended = true;
            }

            return ret;
        }
        catch (const exception& e) {
            if (next != NULL) {
                delete next;
            }

            ret.ended = true;
            ret.err = kytea_err_message(e);

            return ret;
        }
    }
}
