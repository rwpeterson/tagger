@0xd93f59c43d195eec;

using Tags = import "tags.capnp".Tags;

# Publisher-subscriber API
#
# Over-network tag and pattern streaming
# Warning: Tag streaming will likely only work on gigabit lan or localhost
# Note: Conveniently, this follows the capnproto-rust pubsub example
# exactly. We intend this for tags but because of the generic parameter
# we can use it for anything.
# See also https://stackoverflow.com/a/41691580 on different ways to implement
# this in Cap'n Proto (the capnproto-rust example follows the Callback method).

interface Subscription {}

interface Publisher(T) {
    # Drop subscription to signal subscriber is no longer interested in receiving messages
    subscribe @0
        ( subscriber :Subscriber(T)
        , services :ServiceSub
        ) -> (subscription :Subscription);

    # Set channel properties (one property of one channel at a time)
    setInput @1 (s :InputSettings) -> ();
    # Get properties of all channels
    getInputs @2 () -> (s :InputState);
    # Query mode
    queryMode @3 () -> (m :Mode);
    # Set/get global window (logic mode only)
    setWindow @4 (w :UInt32) -> ();
    getWindow @5 () -> (w :UInt32);
}

interface Subscriber(T) {
    # Subscriber should return from this message when it is ready to process the next one
    pushMessage @0 (message :T) -> ();
}

enum Mode {
    timetag @0;
    logic   @1;
}

struct InputState {
    inversionmask @0 :UInt16;
    delays        @1 :List(UInt32);
    thresholds    @2 :List(Float64);
}

struct InputSettings {
    union {
        inversion @0 :ChannelInversion;
        delay     @1 :ChannelDelay;
        threshold @2 :ChannelThreshold;
    }
}

struct ChannelInversion {
    ch  @0 :UInt8;
    inv @1 :Bool;
}

struct ChannelDelay {
    ch  @0 :UInt8;
    del @1 :UInt32;
}

struct ChannelThreshold {
    ch @0 :UInt8;
    th @1 :Float64;
}

struct ServiceSub {
    tagmask  @0 :UInt16 = 0;
    patmasks :union {
        bare     @1 :List(UInt16);
        windowed @2 :List(LogicPattern);
    }
}

struct ServicePub {
    tags @0 :TagPattern;
    pats @1 :List(LogicPattern);
}

struct TagPattern {
    tagmask  @0 :UInt16;
    duration @1 :UInt64;
    tags     @2 :Tags;
}

struct LogicPattern {
    patmask  @0 :UInt16;
    duration @1 :UInt64 = 0;
    count    @2 :UInt64 = 0;
    window   @3 :UInt32 = 0;
}