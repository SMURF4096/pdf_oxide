{
  "targets": [
    {
      "target_name": "pdf_oxide",
      "sources": ["binding.cc"],
      "include_dirs": [
        "<!@(node -p \"require('node-addon-api').include\")"
      ],
      "dependencies": [
        "<!(node -p \"require('node-addon-api').gyp\")"
      ],
      "cflags": ["-Wall", "-Wextra"],
      "cflags_cc": ["-fexceptions"],
      "xcode_settings": {
        "GCC_ENABLE_CPP_EXCEPTIONS": "YES",
        "CLANG_CXX_LIBRARY": "libc++"
      },
      "msvs_settings": {
        "VCCLCompilerTool": {
          "ExceptionHandling": 1
        }
      },
      "libraries": ["-lpdf_oxide"],
      "library_dirs": [
        "<!@(pwd)/../lib"
      ],
      "link_settings": {
        "libraries": ["-lpdf_oxide"],
        "library_dirs": [
          "<!@(pwd)/../lib"
        ]
      }
    }
  ]
}
