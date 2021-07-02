#include "timetag/include/taghelper.h"
#include "timetag/src/lib.rs.h"

/* currently not working !!!
namespace rust {
namespace behavior {

template <typename Try, typename Fail>
static void trycatch(Try &&func, Fail &&fail) noexcept try {
  func();
} catch (const TimeTag::Exception &e) {
  fail(e.GetMessageText().c_str());
}

} // namespace behavior
} // namespace rust
*/

namespace TimeTag {

TimeTagger::TimeTagger()
    : impl(new CTimeTag)
    , c(new ChannelType*)
    , t(new TimeType*)
    {}

auto TimeTagger::open() const -> void {
    impl->Open();
}

auto TimeTagger::close() const -> void {
    impl->Close();
}

auto TimeTagger::calibrate() const -> void {
    impl->Calibrate();
}

auto TimeTagger::read_error_flags() const -> uint32_t {
    return uint32_t(impl->ReadErrorFlags());
}

auto TimeTagger::get_no_inputs() const -> uint8_t {
    return uint8_t(impl->GetNoInputs());
}

auto TimeTagger::get_resolution() const -> double {
    return impl->GetResolution();
}

auto TimeTagger::set_input_threshold(uint8_t input, double voltage) const -> void {
    impl->SetInputThreshold(int(input), voltage);
}

auto TimeTagger::set_inversion_mask(uint32_t mask) const -> void {
    impl->SetInversionMask(int(mask));
}

auto TimeTagger::set_delay(uint8_t input, uint32_t delay) const -> void {
    impl->SetDelay(int(input), int(delay));
}

auto TimeTagger::get_fpga_version() const -> int32_t {
    return int32_t(impl->GetFpgaVersion());
}

auto TimeTagger::set_led_brightness(uint8_t percent) const -> void {
    impl->SetLedBrightness(int(percent));
}

auto TimeTagger::set_fg(uint32_t period, uint32_t high) const -> void {
    impl->SetFG(int(period), int(high));
}

auto TimeTagger::freeze_single_counter() const -> uint64_t {
    return uint64_t(impl->FreezeSingleCounter());
}
auto TimeTagger::get_single_count(uint8_t input) const -> uint32_t {
    return uint32_t(impl->GetSingleCount(int(input)));
}

auto TimeTagger::start_timetags() const -> void {
    impl->StartTimetags();
}

auto TimeTagger::stop_timetags() const -> void {
    impl->StopTimetags();
}

auto TimeTagger::read_tags() const -> std::unique_ptr<std::vector<FfiTag>> {
    int len = impl->ReadTags(*c, *t);
    std::unique_ptr<std::vector<FfiTag>> p = std::make_unique<std::vector<FfiTag>>();
    p->reserve(len);
    for (int i = 0; i < len; i++) {
        /*
         * This aggregate initialization with parentheses inside emplace_back is
         * a C++20 feature supported in gcc and MSVC, but not clang. This is
         * hopefully more performant than separately constructing the
         * element-to-be with {} and copying it into the vector, and importantly
         * allows us to do this without defining a constructor, which lets us
         * use a struct type defined in Rust and exported via CXX.
         */
        p->emplace_back(*(*t + i), *(*c + i));
    }
    return p;
}

auto TimeTagger::use_timetag_gate(bool b) const -> void {
    impl->UseTimetagGate(b);
}

auto TimeTagger::set_gate_width(uint32_t duration) const -> void {
    impl->SetGateWidth(int(duration));
}

auto TimeTagger::use_level_gate(bool b) const -> void {
    impl->UseLevelGate(b);
}

auto TimeTagger::level_gate_active() const -> bool {
    return impl->LevelGateActive();
}

auto TimeTagger::set_filter_min_count(uint32_t min_count) const -> void {
    impl->SetFilterMinCount(int(min_count));
}

auto TimeTagger::set_filter_max_time(uint32_t max_time) const -> void {
    impl->SetFilterMaxTime(int(max_time));
}

auto TimeTagger::set_filter_exception(uint32_t exception) const -> void {
    impl->SetFilterException(int(exception));
}

auto TimeTagger::use_10MHz(bool b) const -> void {
    impl->Use10MHz(b);
}

auto new_time_tagger() -> std::unique_ptr<TimeTagger> {
    return std::make_unique<TimeTagger>();
}

LogicCounter::LogicCounter()
    : impl_tti(new CTimeTag)
    , impl(new CLogic(impl_tti.get()))
    {}

auto LogicCounter::switch_logic_mode() const -> void {
    impl->SwitchLogicMode();
}

auto LogicCounter::set_window_width(uint32_t window) const -> void {
    impl->SetWindowWidth(int(window));
}

auto LogicCounter::set_delay(uint8_t input, uint32_t delay) const -> void {
    impl->SetDelay(int(input), int(delay));
}

auto LogicCounter::read_logic() const -> int64_t {
    return int64_t(impl->ReadLogic());
}

auto LogicCounter::calc_count(uint16_t pos, uint16_t neg) const -> uint32_t {
    return uint32_t(impl->CalcCount(int(pos), int(neg)));
}

auto LogicCounter::calc_count_pos(uint16_t pos) const -> uint32_t {
    return uint32_t(impl->CalcCountPos(int(pos)));
}

auto LogicCounter::get_time_counter() const -> uint64_t {
    return uint64_t(impl->GetTimeCounter());
}

auto LogicCounter::set_output_width(uint8_t width) const -> void {
    impl->SetOutputWidth(int(width));
}

auto LogicCounter::set_output_pattern(uint8_t output, uint16_t pos, uint16_t neg) const -> void {
    impl->SetOutputPattern(int(output), int(pos), int(neg));
}

auto LogicCounter::set_output_event_count(uint32_t events) const -> void {
    impl->SetOutputEventCount(int(events));
}

// Methods for the inner TimeTagger

auto LogicCounter::open() const -> void {
    impl_tti->Open();
}

auto LogicCounter::close() const -> void {
    impl_tti->Close();
}

auto LogicCounter::calibrate() const -> void {
    impl_tti->Calibrate();
}

auto LogicCounter::read_error_flags() const -> uint32_t {
    return uint32_t(impl_tti->ReadErrorFlags());
}

auto LogicCounter::get_no_inputs() const -> uint8_t {
    return uint8_t(impl_tti->GetNoInputs());
}

auto LogicCounter::get_resolution() const -> double {
    return impl_tti->GetResolution();
}

auto LogicCounter::set_input_threshold(uint8_t input, double voltage) const -> void {
    impl_tti->SetInputThreshold(int(input), voltage);
}

auto LogicCounter::set_inversion_mask(uint32_t mask) const -> void {
    impl_tti->SetInversionMask(int(mask));
}

auto LogicCounter::get_fpga_version() const -> int32_t {
    return int32_t(impl_tti->GetFpgaVersion());
}

auto LogicCounter::set_led_brightness(uint8_t percent) const -> void {
    impl_tti->SetLedBrightness(int(percent));
}

auto LogicCounter::set_fg(uint32_t period, uint32_t high) const -> void {
    impl_tti->SetFG(int(period), int(high));
}

auto LogicCounter::use_10MHz(bool b) const -> void {
    impl_tti->Use10MHz(b);
}

std::unique_ptr<LogicCounter> new_logic_counter() {
    return std::make_unique<LogicCounter>();
}

} // namespace TimeTag