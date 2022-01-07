from gspreadWrapper import GspreadWrapper
import gdown
import pandas as pd
import gspread
import json
import re
from time import sleep
from options import Options
from utils import Utils
import os

class PrepareVCAsProfiles():
    def __init__(self):
        self.opt = Options()
        self.utils = Utils()
        self.gspreadWrapper = GspreadWrapper()
        doc = self.gspreadWrapper.gc.open_by_key(self.opt.vcaResponses)
        worksheet = doc.worksheet('Form Responses 1')
        self.responses = worksheet.get_all_records()
        self.vcas = []
        self.fileErrors = []

    def get_valid_filename(self, name):
        s = str(name).strip().replace(' ', '_')
        s = re.sub(r'(?u)[^-\w.]', '', s)
        return s

    def download_from_sheet(self, gfile, docId, localFilename):
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
            df.to_csv('vcas-files/' + localFilename, index=False)
            print("Downloaded successfully: {}".format(gfile))
        except gspread.exceptions.APIError as e:
            dict_error = e.response.json()
            if dict_error['error']['status'] == 'RESOURCE_EXHAUSTED':
                print("Rate limit exceeded. Sleep for 30s...")
                sleep(30)
                print("Retry download...")
                self.download_from_sheet(gfile, docId, localFilename)
            else:
                print("GSheet error downloading: {}".format(gfile))
                self.fileErrors.append(gfile)
                print(e)
        except Exception as e:
            print(e)
            self.fileErrors.append(gfile)
            print("Generic error downloading: {}".format(gfile))


    def downloadFiles(self):
        for response in self.responses:
            gfile = response['Link to your copy of the master assessment sheet ']
            username = response['What is your Ideascale User name?']
            fullName = "{} {}".format(response['Your First Name'], response['Your Last Name'])
            email = response['Email Address']
            localFilename = self.get_valid_filename(username) + '.csv'
            docId = re.findall("[-\w]{25,}", gfile)
            if (len(docId) == 1):
                print("\n######\n")
                if not (os.path.exists('vcas-files/' + localFilename)):
                    print("Downloading: {} from {}".format(gfile, username))
                    docId = docId[0]
                    if ("drive.google.com" in gfile):
                        durl = 'https://drive.google.com/uc?id=' + docId
                        gdown.download(durl, 'vcas-files/' + localFilename, quiet=True)
                    else:
                        self.download_from_sheet(gfile, docId, localFilename)
                else:
                    print("Already downloaded: {} from {}".format(gfile, username))
                vca = {
                    "name": fullName,
                    "vca_link": gfile,
                    "vca_file": localFilename,
                    "ca_id": "",
                    "email": email,
                    "proposals": [],
                    "campaigns_as_proposers": [],
                    "userName": username
                }
                self.vcas.append(vca)
            else:
                print("Not valid Google doc/sheet found.")

        with open('download-errors.json', 'w') as f:
            json.dump(self.fileErrors, f)

        with open('vcas.json', 'w') as f:
            json.dump(self.vcas, f, indent=4)

prepareVCAsProfiles = PrepareVCAsProfiles()
prepareVCAsProfiles.downloadFiles()
