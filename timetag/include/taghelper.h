#pragma once
#include "rust/cxx.h"
#include "CLogic.h"
#include <memory>
#include <vector>

/* currently not working !!!
// This overrides CXX's default exception -> Result behavior, since CTimeTag
// does not use std::exception
namespace rust {
namespace behavior {

template <typename Try, typename Fail>
static void trycatch(Try &&func, Fail &&fail) noexcept;

} // namespace behavior
} // namespace rust
*/

namespace TimeTag {

struct FfiTag;

class TimeTagger {
public:
    TimeTagger();
    auto open() const -> void;
    auto close() const -> void;
    auto calibrate() const -> void;
    auto read_error_flags() const -> uint32_t;
    //auto get_error_text(int32_t flags) const -> std::unique_ptr<std::string>
    auto get_no_inputs() const -> uint8_t;
    auto get_resolution() const -> double;
    auto set_input_threshold(uint8_t input, double voltage) const -> void;
    auto set_inversion_mask(uint32_t mask) const -> void;
    auto set_delay(uint8_t input, uint32_t delay) const -> void;
    auto get_fpga_version() const -> int32_t; // return value is debug only
    auto set_led_brightness(uint8_t percent) const -> void;
    auto set_fg(uint32_t period, uint32_t high) const -> void;
    auto freeze_single_counter() const -> uint64_t;
    auto get_single_count(uint8_t input) const -> uint32_t;
    auto start_timetags() const -> void;
    auto stop_timetags() const -> void;
    auto read_tags() const -> std::unique_ptr<std::vector<FfiTag>>;
    auto use_timetag_gate(bool b) const -> void;
    auto set_gate_width(uint32_t duration) const -> void;
    auto use_level_gate(bool b) const -> void;
    auto level_gate_active() const -> bool;
    auto set_filter_min_count(uint32_t min_count) const -> void;
    auto set_filter_max_time(uint32_t max_time) const -> void;
    auto set_filter_exception(uint32_t exception) const -> void;
    auto use_10MHz(bool b) const -> void;

    // missing from CTimeTag.lib: TagsPresent()

private:
    std::unique_ptr<CTimeTag> impl;
    std::unique_ptr<ChannelType*> c;
    std::unique_ptr<TimeType*> t;
};

std::shared_ptr<TimeTagger> new_time_tagger();

class LogicCounter {
public:
    LogicCounter();

    // Logic mode methods
    auto switch_logic_mode() const -> void;
    auto set_window_width(uint32_t window) const -> void;
    auto set_delay(uint8_t input, uint32_t delay) const -> void;
    auto read_logic() const -> int64_t; // return value is debug only
    auto calc_count(uint16_t pos, uint16_t neg) const -> uint32_t;
    auto calc_count_pos(uint16_t pos) const -> uint32_t;
    auto get_time_counter() const -> uint64_t;
    auto set_output_width(uint8_t width) const -> void;
    auto set_output_pattern(uint8_t output, uint16_t pos, uint16_t neg) const -> void;
    auto set_output_event_count(uint32_t events) const -> void;

    // Time Tagger methods to call on the inner object (usually before switching to logic mode)
    auto open() const -> void;
    auto close() const -> void;
    auto calibrate() const -> void;
    auto read_error_flags() const -> uint32_t;
    auto get_no_inputs() const -> uint8_t;
    auto get_resolution() const -> double;
    auto set_input_threshold(uint8_t input, double voltage) const -> void;
    auto set_inversion_mask(uint32_t mask) const -> void;
    // set_delay removed
    auto get_fpga_version() const -> int32_t; // return value is debug only
    auto set_led_brightness(uint8_t percent) const -> void;
    auto set_fg(uint32_t period, uint32_t high) const -> void;
    // singles counter methods removed
    // timetag methods removed
    auto use_10MHz(bool b) const -> void;

private:
    std::unique_ptr<CTimeTag> impl_tti;
    std::unique_ptr<CLogic> impl;
};

std::shared_ptr<LogicCounter> new_logic_counter();

} // namespace TimeTag