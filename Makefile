# Based on https://stackoverflow.com/a/52036564 which is well worth reading!

CXXFLAGS := -std=c++20 # use the 2020 version of the C++ standard
CXXFLAGS += -g # generate debugging information
CXXFLAGS += -Wall # enable most warnings
CXXFLAGS += -Wextra # enable extra warnings
CXXFLAGS += -Werror # treat all warnings as errors
CXXFLAGS += -fsanitize=address # enable address sanitization
CXXFLAGS += -static-libasan # statically link with Address Sanitizer
CXXFLAGS += -O0 # perform minimal optimisations
CXXFLAGS += -rdynamic # to get more helpful traces when debugging
CXXFLAGS += --coverage # enable code coverage
CXXFLAGS += -I include # look for header files in the `include` directory

SOURCES := $(wildcard src/*.cpp) # all .cpp files are to be considered source files
DEPENDENCIES := $(patsubst src/%.cpp,build/%.d,$(SOURCES))

OBJECTS := $(patsubst src/%.cpp,build/%.o,$(SOURCES))
OBJECTS += build/parser.tab.o build/lexer.yy.o

.PHONY: default clean coverage

default: bin/c_compiler

bin/c_compiler:
	@mkdir -p bin
	cd rustc_compiler && cargo build --release
	cp rustc_compiler/target/release/rustc_compiler bin/c_compiler
	chmod +x bin/c_compiler

-include $(DEPENDENCIES)

build/%.o: src/%.cpp Makefile
	@mkdir -p $(@D)
	g++ $(CXXFLAGS) -MMD -MP -c $< -o $@

build/parser.tab.cpp build/parser.tab.hpp: src/parser.y
	@mkdir -p build
	bison -v -d src/parser.y -o build/parser.tab.cpp

build/lexer.yy.cpp: src/lexer.flex build/parser.tab.hpp
	@mkdir -p build
	flex -o build/lexer.yy.cpp src/lexer.flex

coverage:
	@mkdir -p coverage
	cd rustc_compiler && cargo install grcov
	cd rustc_compiler && RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="coverage-%p-%m.profraw" cargo test --release
	cd rustc_compiler && grcov . --binary-path ./target/release/ -s . -t html --branch --ignore-not-existing -o ../coverage/
	@find . -name "*.profraw" -delete

clean:
	@rm -rf coverage/
	@rm -rf bin/
	cd rustc_compiler && cargo clean
