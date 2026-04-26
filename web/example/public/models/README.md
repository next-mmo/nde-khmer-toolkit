Local browser TTS models
========================

Place browser-compatible Transformers.js/ONNX models here.

For Khmer MMS TTS, the app looks for:

  public/models/facebook/mms-tts-khm/
    config.json
    preprocessor_config.json
    tokenizer_config.json
    vocab.json
    onnx/
      model.onnx
      model_quantized.onnx

The upstream facebook/mms-tts-khm checkpoint is a PyTorch/Transformers model, not a browser
ONNX package. Convert it with Optimum/Transformers.js tooling before static deployment.
