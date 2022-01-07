from gspreadWrapper import GspreadWrapper
import gdown
import pandas as pd
import gspread
import json
import re
from time import sleep

class DownloadCsvFromDrive():
    def __init__(self):
        self.gspreadWrapper = GspreadWrapper()
        self.fileList = []

        self.fileErrors = []

    def get_valid_filename(self, name):
        s = str(name).strip().replace(' ', '_')
        s = re.sub(r'(?u)[^-\w.]', '', s)
        return s

    def download_from_sheet(self, gfile, docId):
        try:
            doc = self.gspreadWrapper.gc.open_by_key(docId)
            sheets = doc.worksheets()
            sheetsTitles = [x.title for x in sheets]
            if ('Assessments' in sheetsTitles):
                sheet = doc.worksheet("Assessments")
            else:
                sheet = doc.get_worksheet(0)
            df = pd.DataFrame(sheet.get_all_records())
            df.fillna('', inplace=True)
            df.to_csv('proposers-files/' + self.get_valid_filename(doc.title) + '.csv', index=False)
            print("Downloaded successfully: {}".format(gfile))
        except gspread.exceptions.APIError as e:
            dict_error = e.response.json()
            if dict_error['error']['status'] == 'RESOURCE_EXHAUSTED':
                print("Rate limit exceeded. Sleep for 30s...")
                sleep(30)
                print("Retry download...")
                self.download_from_sheet(gfile, docId)
            else:
                print("GSheet error downloading: {}".format(gfile))
                self.fileErrors.append(gfile)
                print(e)
        except Exception as e:
            print(e)
            self.fileErrors.append(gfile)
            print("Generic error downloading: {}".format(gfile))


    def downloadFiles(self):
        for gfile in self.fileList:
                print("\n######\n")
                print("Downloading: {}".format(gfile))
                docId = re.findall("[-\w]{25,}", gfile)
                if (len(docId) == 1):
                    docId = docId[0]
                    if ("drive.google.com" in gfile):
                        durl = 'https://drive.google.com/uc?id=' + docId
                        gdown.download(durl, 'proposers-files/' + docId + '.csv', quiet=True)
                    else:
                        self.download_from_sheet(gfile, docId)
                else:
                    print("Not valid Google doc/sheet found.")

        with open('download-errors.json', 'w') as f:
            json.dump(self.fileErrors, f)

down = DownloadCsvFromDrive()
down.downloadFiles()
