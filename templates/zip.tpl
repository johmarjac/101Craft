// 101Craft template: ZIP archive
LittleEndian();

typedef struct {
    uint   signature <format=hex>;
    ushort version;
    ushort flags <format=hex>;
    ushort compression;
    ushort modTime;
    ushort modDate;
    uint   crc32 <format=hex>;
    uint   compressedSize;
    uint   uncompressedSize;
    ushort nameLength;
    ushort extraLength;
    char   fileName[nameLength];
    if (extraLength > 0)
        uchar extra[extraLength];
    if (compressedSize > 0)
        uchar data[compressedSize];
} LOCALFILE;

typedef struct {
    uint   signature <format=hex>;
    ushort versionMadeBy;
    ushort versionNeeded;
    ushort flags <format=hex>;
    ushort compression;
    ushort modTime;
    ushort modDate;
    uint   crc32 <format=hex>;
    uint   compressedSize;
    uint   uncompressedSize;
    ushort nameLength;
    ushort extraLength;
    ushort commentLength;
    ushort diskStart;
    ushort internalAttributes;
    uint   externalAttributes <format=hex>;
    uint   localHeaderOffset <format=hex>;
    char   fileName[nameLength];
    if (extraLength > 0)
        uchar extra[extraLength];
    if (commentLength > 0)
        char comment[commentLength];
} DIRENTRY;

typedef struct {
    uint   signature <format=hex>;
    ushort diskNumber;
    ushort centralDirDisk;
    ushort entriesOnDisk;
    ushort totalEntries;
    uint   centralDirSize;
    uint   centralDirOffset <format=hex>;
    ushort commentLength;
    if (commentLength > 0)
        char comment[commentLength];
} ENDRECORD;

local uint tag;
while (FTell() + 4 <= FileSize()) {
    tag = ReadUInt(FTell());
    if (tag == 0x04034b50)
        LOCALFILE file;
    else if (tag == 0x02014b50)
        DIRENTRY dirEntry;
    else if (tag == 0x06054b50)
        ENDRECORD endRecord;
    else
        break;
}
