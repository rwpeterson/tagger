@0xd93f59c43d195eec;

using Tags = import "tags.capnp".Tags;

struct Job {
    id         @0 :UInt64        =0;     # Job id
    patterns   @1 :List(UInt16);         # List of bitmasks of channels
    events     @2 :List(UInt64);         # List of event counts in these patterns
    window     @3 :Int64;                # Coincidence window (internal units, see Resolution)
    duration   @4 :UInt64;               # Elapsed time (tick units, 5 ns)
    finished   @5 :Bool          =false;
    starttag   @6 :Int64;
    stoptag    @7 :Int64;
    meta       @8 :JobMeta       =(submission = void);
    resolution @9 :Resolution    =norm;
    handle    @10 :Text          ="";    # For server impl serial/parallel job queueing
}

struct JobMeta {
    union {
        submission @0 :Void;
        ok         @1 :Void;
        err        @2 :Text;
    }
}

enum Resolution {
    norm @0; # 156.25 ps
    fast @1; # 78.125 ps
}

enum JobStatus {
    badid     @0; # JobID not recognized
    waiting   @1; # Job queued, please wait
    cancelled @2; # Job cancelled
    ready     @3; # Job is ready
    error     @4; # Out-of-band error
    badjob    @5; # Job params out of bounds (job's fault)
    refused   @6; # Job is refused by server (not job's fault)
    claimed   @7; # Job already returned to client
}

struct JobPayload {
    union {
        badquery @0 :JobStatus;
        payload @1 :Job;
    }
}

struct JobSubmission {
    union {
        badsub @0 :JobStatus;
        jobid @1 :UInt64; # Successful submission
    }
}

interface Tagger {
    savetags     @0
        ( filename  :Text
        , chans     :List(UInt8)
        , duration  :UInt64
        ) -> (jobid :UInt64);

    submitjob    @1 (job   :Job)    -> (sub     :JobSubmission);

    queryjobdone @2 (jobid :UInt64) -> (ret     :JobStatus);

    getresults   @3 (jobid :UInt64) -> (payload :JobPayload);
}

# Over-network tag and pattern streaming
# Warning: This will likely only work on gigabit lan or localhost in tag mode
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
        ) -> (subscription: Subscription);
}

interface Subscriber(T) {
    # Subscriber should return from this message when it is ready to process the next one
    pushMessage @0 (message :T) -> ();
}

struct ServiceSub {
    tagmask  @0 :UInt16;
    patmasks @1 :List(UInt16);
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
    mask     @0 :UInt16;
    count    @1 :UInt64;
    duration @2 :UInt64;
}