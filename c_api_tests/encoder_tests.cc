/*
 * Copyright 2025 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <cstring>
#include <iostream>

#include "avif/avif.h"
#include "gtest/gtest.h"
#include "testutil.h"

namespace avif {
namespace {

// Used to pass the data folder path to the GoogleTest suites.
const char* data_path = nullptr;

std::string GetFilename(const char* file_name) {
  return std::string(data_path) + file_name;
}

DecoderPtr CreateDecoder(const avifRWData& encoded) {
  DecoderPtr decoder(avifDecoderCreate());
  if (decoder == nullptr ||
      avifDecoderSetIOMemory(decoder.get(), encoded.data, encoded.size) !=
          AVIF_RESULT_OK) {
    return nullptr;
  }
  return decoder;
}

TEST(BasicTest, EncodeDecode) {
  ImagePtr image = testutil::CreateImage(/*width=*/12, /*height=*/34,
                                         /*depth=*/8, AVIF_PIXEL_FORMAT_YUV420,
                                         AVIF_PLANES_ALL, AVIF_RANGE_FULL);
  ASSERT_NE(image, nullptr);
  testutil::FillImageGradient(image.get(), /*offset=*/0);

  EncoderPtr encoder(avifEncoderCreate());
  encoder->quality = 60;
  encoder->speed = 10;
  ASSERT_NE(encoder, nullptr);
  AvifRwDataPtr encoded(new avifRWData());
  avifResult result =
      avifEncoderWrite(encoder.get(), image.get(), encoded.get());
  ASSERT_EQ(result, AVIF_RESULT_OK);
  auto decoder = CreateDecoder(*encoded);
  ASSERT_NE(decoder, nullptr);
  ASSERT_EQ(avifDecoderParse(decoder.get()), AVIF_RESULT_OK);
  ASSERT_EQ(avifDecoderNextImage(decoder.get()), AVIF_RESULT_OK);
  ASSERT_GT(testutil::GetPsnr(*image, *decoder->image, /*ignore_alpha=*/false),
            40.0);
}

}  // namespace
}  // namespace avif

int main(int argc, char** argv) {
  ::testing::InitGoogleTest(&argc, argv);
  if (argc != 2) {
    std::cerr << "There must be exactly one argument containing the path to "
                 "the test data folder"
              << std::endl;
    return 1;
  }
  avif::data_path = argv[1];
  return RUN_ALL_TESTS();
}
