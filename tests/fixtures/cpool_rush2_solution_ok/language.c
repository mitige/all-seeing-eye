/*
** EPITECH PROJECT, 2026
** cpool_rush2
** File description:
** language guessing by nearest letter-frequency profile
*/

/*
** Reference profiles : the classic per-language letter frequencies
** (English, French, German, Spanish), scaled by 100 and kept as
** integers — 1270 means 12.70 percent. The guess is the language whose
** profile is the closest to the text frequencies, using the sum of
** absolute differences over the 26 letters.
*/

#include "rush2.h"

static int const *reference_english(void)
{
    static const int table[26] = {817, 149, 278, 425, 1270, 223, 202, 609,
        697, 15, 77, 403, 241, 675, 751, 193, 10, 599, 633, 906, 276, 98,
        236, 15, 197, 7};

    return (table);
}

static int const *reference_french(void)
{
    static const int table[26] = {764, 90, 326, 367, 1472, 107, 87, 74,
        753, 61, 5, 546, 297, 710, 580, 252, 136, 669, 795, 724, 631, 184,
        7, 43, 30, 33};

    return (table);
}

static int const *reference_german(void)
{
    static const int table[26] = {652, 189, 273, 508, 1640, 166, 301, 458,
        655, 27, 142, 344, 253, 978, 259, 67, 2, 700, 727, 615, 417, 85,
        192, 3, 4, 113};

    return (table);
}

static int const *reference_spanish(void)
{
    static const int table[26] = {1253, 142, 468, 586, 1368, 69, 101, 70,
        625, 44, 2, 497, 315, 671, 868, 251, 88, 687, 798, 463, 393, 90,
        1, 22, 90, 52};

    return (table);
}

static int const *reference_frequencies(int lang)
{
    if (lang == 1)
        return (reference_french());
    if (lang == 2)
        return (reference_german());
    if (lang == 3)
        return (reference_spanish());
    return (reference_english());
}

static void fill_frequencies(char const *text, int *freq)
{
    int i = 0;

    while (i < 26) {
        freq[i] = 0;
        i = i + 1;
    }
    i = 0;
    while (text[i] != '\0') {
        if (is_letter(text[i]))
            freq[to_lower(text[i]) - 'a'] = freq[to_lower(text[i]) - 'a'] + 1;
        i = i + 1;
    }
}

static int distance_to(int const *freq, int const *ref, int total)
{
    int dist = 0;
    int centi = 0;
    int i = 0;

    while (i < 26) {
        centi = 0;
        if (total > 0)
            centi = freq[i] * 10000 / total;
        if (centi > ref[i])
            dist = dist + centi - ref[i];
        else
            dist = dist + ref[i] - centi;
        i = i + 1;
    }
    return (dist);
}

static char const *name_of(int lang)
{
    if (lang == 1)
        return ("French");
    if (lang == 2)
        return ("German");
    if (lang == 3)
        return ("Spanish");
    return ("English");
}

char const *guess_language_name(char const *text, int total)
{
    int freq[26];
    int best = 0;
    int best_dist = -1;
    int dist = 0;
    int lang = 0;

    fill_frequencies(text, freq);
    while (lang < 4) {
        dist = distance_to(freq, reference_frequencies(lang), total);
        if (best_dist < 0 || dist < best_dist) {
            best_dist = dist;
            best = lang;
        }
        lang = lang + 1;
    }
    return (name_of(best));
}
