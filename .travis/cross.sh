#!/bin/sh

set -ex

mkdir -p $HOME/cached/bin
PATH=$HOME/cached/bin:$PATH


sudo apt-get install git

case "$PLATFORM" in
    "raspbian")
        which cargo-dinghy || cargo install --debug --root $HOME/cached cargo-dinghy
        [ -e $HOME/cached/raspitools ] || git clone https://github.com/raspberrypi/tools $HOME/cached/raspitools
        TOOLCHAIN=$HOME/cached/raspitools/arm-bcm2708/arm-rpi-4.9.3-linux-gnueabihf
        export RUSTC_TRIPLE=arm-unknown-linux-gnueabihf
        rustup target add $RUSTC_TRIPLE
        echo "[platforms.$PLATFORM]\nrustc_triple='$RUSTC_TRIPLE'\ntoolchain='$TOOLCHAIN'" > $HOME/.dinghy.toml
        cargo dinghy --platform $PLATFORM build --release -p tract
    ;;
    "aarch64")
        sudo apt-get -y install binutils-aarch64-linux-gnu gcc-4.8-aarch64-linux-gnu
        export RUSTC_TRIPLE=aarch64-unknown-linux-gnu
        rustup target add $RUSTC_TRIPLE
        export TARGET_CC=aarch64-linux-gnu-gcc-4.8
        export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc-4.8
        export BLIS_SRC_GIT_URL=https://github.com/kali/blis
        export BLIS_SRC_GIT_BRANCH=arm32vfp
        export BLIS_SRC_ARCH_OVERRIDE=cortexa53
        export BLIS_SRC_OVERRIDE_STATIC=1
        (cd cli; cargo build --target $RUSTC_TRIPLE --release --features blis)
    ;;
    *)
esac

if [ -n "$AWS_ACCESS_KEY_ID" ]
then
    TASK_NAME=`.travis/make_bundle.sh`
    aws s3 cp $TASK_NAME.tgz s3://tract-ci-builds/tasks/$PLATFORM/$TASK_NAME.tgz
fi