// 101Craft template: Windows program (EXE/DLL, PE format)
LittleEndian();

typedef struct {
    char   e_magic[2];
    ushort e_cblp;
    ushort e_cp;
    ushort e_crlc;
    ushort e_cparhdr;
    ushort e_minalloc;
    ushort e_maxalloc;
    ushort e_ss;
    ushort e_sp;
    ushort e_csum;
    ushort e_ip;
    ushort e_cs;
    ushort e_lfarlc;
    ushort e_ovno;
    ushort e_res[4];
    ushort e_oemid;
    ushort e_oeminfo;
    ushort e_res2[10];
    uint   e_lfanew <format=hex, comment="Start of the PE header">;
} DOS_HEADER;

typedef enum <ushort> {
    MACHINE_I386  = 0x14c,
    MACHINE_ARM   = 0x1c0,
    MACHINE_AMD64 = 0x8664,
    MACHINE_ARM64 = 0xAA64
} MACHINE;

typedef struct {
    MACHINE Machine;
    ushort  NumberOfSections;
    uint    TimeDateStamp;
    uint    PointerToSymbolTable;
    uint    NumberOfSymbols;
    ushort  SizeOfOptionalHeader;
    ushort  Characteristics <format=hex>;
} FILE_HEADER;

typedef struct {
    char   Name[8];
    uint   VirtualSize <format=hex>;
    uint   VirtualAddress <format=hex>;
    uint   SizeOfRawData <format=hex>;
    uint   PointerToRawData <format=hex>;
    uint   PointerToRelocations;
    uint   PointerToLinenumbers;
    ushort NumberOfRelocations;
    ushort NumberOfLinenumbers;
    uint   Characteristics <format=hex>;
} SECTION_HEADER;

DOS_HEADER dosHeader;
if (dosHeader.e_lfanew > 0 && dosHeader.e_lfanew + 24 < FileSize()) {
    FSeek(dosHeader.e_lfanew);
    char signature[4];
    FILE_HEADER fileHeader;
    if (fileHeader.SizeOfOptionalHeader > 2) {
        ushort optionalMagic <format=hex>;
        uchar  optionalHeader[fileHeader.SizeOfOptionalHeader - 2];
    }
    if (fileHeader.NumberOfSections > 0)
        SECTION_HEADER sections[fileHeader.NumberOfSections];
}
