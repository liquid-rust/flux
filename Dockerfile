# For a demo container:
# docker build --tag=flux .
# To run tests:
# docker build --target=test .

# NOTE: liquid-fixpoint and z3 versions are not pinned,
# watch out for breaking changes

# uses official haskell image containing cabal and stack by default
ARG FIXPOINT_BUILDER="docker.io/library/haskell:9.10.1-bullseye"

# ARG BASE_IMAGE requirements:
# - debian-based because it uses apt-get to install z3
# - cargo is available
ARG BASE_IMAGE="docker.io/library/rust"

FROM $FIXPOINT_BUILDER as fixpoint-builder
WORKDIR /

RUN \
  git clone --single-branch \
  https://github.com/ucsd-progsys/liquid-fixpoint.git \
  && cd /liquid-fixpoint \
  && cabal update -v \
  && cabal install

FROM $BASE_IMAGE AS with-deps

COPY --from=fixpoint-builder /root/.local/bin/fixpoint /usr/bin/

ENV Z3_VERSION="4.13.4"
RUN \
  if [ "$(uname -m)" = 'arm64' ]; then \ 
    ARCH_COMPONENT="arm64"; \
    GLIBC="2.34"; \
  else \
    ARCH_COMPONENT="x64"; \
    GLIBC="2.35"; \
  fi \
  && ADDR="https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/z3-${Z3_VERSION}-${ARCH_COMPONENT}-glibc-${GLIBC}.zip" \
  && curl -L -o z3.zip "$ADDR" \
  && unzip z3.zip -d ~/ && rm z3.zip && ln -s ~/z3*/bin/z3 /usr/bin/z3

FROM with-deps AS flux-builder
COPY . /flux
WORKDIR /flux
RUN cargo xtask install

FROM flux-builder AS test

WORKDIR /flux
RUN cargo xtask test

# To have flux source available from inside the container, one can substitute `with-deps` with `flux-builder`,
# but if you want the changes to persist across containers (which is what's usually desired),
# then a bound volume with the flux project is a better solution 
FROM with-deps AS final
LABEL org.opencontainers.image.title="Flux - Liquid Types for Rust"
LABEL org.opencontainers.image.description="Docker image to build and test Flux"
LABEL org.opencontainers.image.vendor="ucsd-progsys"
LABEL org.opencontainers.image.source="https://github.com/flux-rs/flux"
LABEL org.opencontainers.image.licenses="MIT"

COPY --from=flux-builder \
  /usr/local/cargo/bin/rustc-flux \
  /usr/local/cargo/bin/cargo-flux \
  /usr/local/cargo/bin/
COPY --from=flux-builder /root/.flux /root/.flux